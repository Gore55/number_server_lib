#![allow(dead_code, unused)]
pub mod test;

use std::sync::Arc;
use std::collections::HashSet;
use tokio::fs::OpenOptions;
use tokio::sync::{Mutex, mpsc::{UnboundedReceiver, UnboundedSender}, mpsc};
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncBufReadExt, Lines, BufReader, BufWriter, AsyncWriteExt};
use std::net::{Ipv4Addr, SocketAddrV4};
use rand::thread_rng;
use rand::prelude::*;
use std::io::prelude::*;

// use tokio::sync::{Mutex, mpsc};
// use std::sync::Arc;

// use number_server::{start_listening, start_logger, start_reporter, AppStatus};


fn lines_from_file() -> Vec<String> {
    let file = std::fs::File::open("test_numbers.log").expect("no such file");
    let buf = std::io::BufReader::new(file);
    buf.lines()
        .map(|l| l.expect("Could not parse line"))
        .collect()
}

async fn send_test(list: Vec<String>, number: u8) {

    let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(500));
    
    let stream = TcpStream::connect("127.0.0.1:4000").await.unwrap();
    println!("Client {} connected", number);
    let mut writer = BufWriter::new(stream);

    
    for x in list {
        interval.tick().await;
        let stringed = format!("{:0>9}\n", x);
        // println!("{:?}", stringed);
        &writer.write(&mut stringed.as_bytes()).await.unwrap();
        match writer.flush().await{
            Ok(_) => {},
            Err(_) => {
                println!("Client {} disconnected", number);
                break
            },
        };
    }


}

async fn start_server() {

    let status = Arc::new(Mutex::new(AppStatus::new()));
    let (tx, rx) = mpsc::unbounded_channel();

    let listener = start_listening(status.clone(), tx.clone(), 4000);
    let logger = start_logger(rx);
    let reporter = start_reporter(status.clone());

    tokio::join!(
        listener,
        logger,
        reporter
    ); 

}

const CERO: &str = "0";

pub struct AppStatus {
    total_unique: usize,
    n_duplicates: usize,
    n_new_unique: usize,
    n_clients: usize,
    terminate: bool,
}

impl AppStatus {
    pub fn new() -> Self {
        AppStatus {
            total_unique: 0,
            n_duplicates: 0,
            n_new_unique: 0,
            n_clients: 0,
            terminate: false,
        }
    }
    fn add_unique(&mut self) {
        self.n_new_unique += 1;
        self.total_unique += 1;
    }
    fn add_duplicate(&mut self) {
        self.n_new_unique += 1;
        self.n_duplicates += 1;
    }
    fn cero_(&mut self) {
        self.n_duplicates = 0;
        self.n_new_unique = 0;
    }
    fn status_print(&mut self) {
        println!("Received {}, {} duplicates. Unique total: {}", self.n_new_unique, self.n_duplicates, self.total_unique);
        self.cero_();
    }
}

enum PossibleOutcome {
    Okay(u32),
    Bad,
    Terminate
}


fn process_line(line: &str) -> PossibleOutcome {

    match line {

        "terminate" => return PossibleOutcome::Terminate,

        _ => {

            if line.len() == 9 {

                let mut stripped = String::from(line);                               
                while stripped.starts_with(CERO) { stripped.remove(0); };
                
                if let Ok(number) = stripped.parse::<u32>() {
                    
                    return PossibleOutcome::Okay(number)
                    
                } else {
                    
                    return PossibleOutcome::Bad

                };

            } else {

                return PossibleOutcome::Bad

            }
        }
    }
}

pub async fn start_listening(status: Arc<Mutex<AppStatus>>, tx: UnboundedSender<u32>, port: u16) {
    
    let listener = TcpListener::bind(SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port)).await.unwrap();
    println!("Now listening at: {}", listener.local_addr().unwrap());
    let existing_numbers: Arc<Mutex<HashSet<u32>>> = Arc::new(Mutex::new(HashSet::new()));

    tokio::spawn(async move {

        loop {
            
            let terminate = status.lock().await.terminate;

            if !terminate {

                match listener.accept().await {
                    
                    Ok((mut peer, _)) => {
                        let n_connections = status.lock().await.n_clients;
                        if n_connections > 4 {
                            println!("Too many clients");
                            peer.shutdown();

                        } else {

                            {
                                status.lock().await.n_clients += 1;
                            }
                            
                            let log_sender = tx.clone();
                            let ref_ex = existing_numbers.clone();
                            let ref_status = status.clone();
                            
                            tokio::spawn(async move {
                                
                                let mut reader = BufReader::new(peer.split().0);
                                
                                loop {

                                    let mut line = String::new();
                                    if let Ok(size) = reader.read_line(&mut line).await {
                                        
                                        if size == 0 {

                                            peer.shutdown();
                                            ref_status.lock().await.n_clients -= 1;
                                            
                                            break
                                        };
                                        
                                        line = line[0..line.len() - 1].to_string();

                                        match process_line(&line) {

                                            PossibleOutcome::Okay(number) => {

                                                let mut locked_ex = ref_ex.lock().await;

                                                if locked_ex.get(&number).is_none() {

                                                    ref_status.lock().await.add_unique();
                                                    locked_ex.insert(number);
                                                    log_sender.send(number).expect("Broken pipe");

                                                } else {

                                                    ref_status.lock().await.add_duplicate();
                                                }

                                            },

                                            PossibleOutcome::Bad => {

                                                peer.shutdown();
                                                ref_status.lock().await.n_clients -= 1;
                                                    
                                                break

                                            },

                                            PossibleOutcome::Terminate => {

                                                ref_status.lock().await.terminate = true;

                                            }
                                        }
                                    }
                                    if ref_status.lock().await.terminate {

                                        peer.shutdown();
                                        break
                                        
                                    }
                                }
                            });
                        }
                    },
    
                    Err(e) => {
                        eprintln!("Error accepting new connection: {}", e);
                    }
                }
                    
            }
        }    
    });

}

pub async fn start_logger(mut rx: UnboundedReceiver<u32>) {
    
    let log_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(true) 
        .open("numbers.log").await.unwrap();   

    let mut writer = BufWriter::new(log_file);

    while let Some(number) = rx.recv().await {

        let mut stringed = number.to_string();
        stringed.push_str("\n");
        writer.write_all(stringed.as_bytes()).await;
        writer.flush().await.expect("Failed to write to log file");

    }

}

pub async fn start_reporter(status: Arc<Mutex<AppStatus>>) {
    
    let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(10));
    
    loop {

        interval.tick().await;
        status.lock().await.status_print();
        println!("Clients connected: {}", status.lock().await.n_clients);

        if status.lock().await.terminate {

            println!("Terminated by client");
            break

        } 

    }

}
