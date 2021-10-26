use crate::{start_server, send_test, lines_from_file};
use tokio::net::TcpStream;
use tokio::io::{AsyncWriteExt, BufWriter};

#[tokio::test]
pub async fn all() -> Result<(), Box<dyn std::error::Error>> {
    
    //Load test number list
    let list = lines_from_file();
    
    //Start number reader/logger server
    tokio::spawn(async move {
        start_server().await;
    });

    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

    for i in 1..=2 {
        //Start 2 clients that send numbers from the list without stopping
        let listo = list.clone();
        tokio::spawn(async move {
            send_test(listo, i).await;
        });
    };

    for i in 3..=4 {
        //Start 2 clients that send numbers from the list on a timer
        let listo = list.clone();
        tokio::spawn(async move {
            if let Err(_) = tokio::time::timeout(tokio::time::Duration::from_secs(80), send_test(listo, i)).await {
                println!("Client {} disconnected", i);
            };
        });
    };

    //Wait a few cycles
    tokio::time::sleep(tokio::time::Duration::from_secs(50)).await;

    tokio::spawn(async move {
        if let Err(_) = tokio::time::timeout(tokio::time::Duration::from_secs(5), send_test(list, 5)).await {
            println!("Client 5 disconnected");
        };
    });

    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;   
    
    //Try to connect but fail because there are already five clients
    tokio::spawn(async {
        
        println!("Client 6 trying to connect");
        let stream = TcpStream::connect("127.0.0.1:4000").await.expect("Failed to connect");
        let mut writer = BufWriter::new(stream);
        
        let stringed = format!("712731723\n");
        
        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;   
        
        writer.write(&mut stringed.as_bytes()).await;
        if writer.flush().await.is_err() {
            println!("Client 6 can't send to server");
        };
        
    });

    tokio::time::sleep(tokio::time::Duration::from_secs(10)).await;   

    //Try to send wrong format
    tokio::spawn(async {
        
        let stream = TcpStream::connect("127.0.0.1:4000").await.expect("Failed to connect");
        println!("Client 7 trying to send wrong format");
        let mut writer = BufWriter::new(stream);
        
        let stringed = format!("AA73C723\n");
        
        loop {
            writer.write(&mut stringed.as_bytes()).await;
            if writer.flush().await.is_err() {
                println!("Client 7 can't send to server again");
                break
            };
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;   
        }
        
    });
    
    tokio::time::sleep(tokio::time::Duration::from_secs(35)).await;
    //Connect to send terminate command but wait to test connection reject
    tokio::spawn(async {

        let stream = TcpStream::connect("127.0.0.1:4000").await.expect("Failed to connect");
        println!("Final client connected");
        let mut writer = BufWriter::new(stream);

        let stringed = format!("terminate\n");
        println!("Sending: {:?}", stringed);
        &writer.write(&mut stringed.as_bytes()).await.expect("Failed to write");
        writer.flush().await.expect("Failed to flush");

    });

    tokio::time::sleep(tokio::time::Duration::from_secs(20)).await;

    Ok(())
    
}