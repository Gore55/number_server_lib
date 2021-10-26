# number_server_lib

This test will show all the functionalities of the server:

- Connect up to 5 clients
- Receive only 9 digits numbers, disconnecting clients that send wrong formatted numbers
- Shutdown at terminate command
- Write only unique numbers to numbers.log
- Print report message each 10 seconds

cd ~/number_server_lib
cargo test -- --nocapture

(This is only intended for funcionality testing, the best performance is achieved with the executable)
