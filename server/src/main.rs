use std::io::{ErrorKind, Read, Write};
use std::net::{TcpListener};
use std::sync::{mpsc, Mutex, Arc};
use std::thread;

/* Address to server. */
const SERVER_ADDR: &str = "127.0.0.1:6000";

/* Max message size in characters. */
const MSG_SIZE: usize = 32;

/**
 * Sleep (current thread) for 100 milliseconds.
 */
fn sleep() {
    thread::sleep(::std::time::Duration::from_millis(100));
}

fn main() {

    // connect to server
    let server = match TcpListener::bind(SERVER_ADDR) {
        Ok(_client) => {
            println!("Opened server at: {}", SERVER_ADDR);
            _client
        },
        Err(_) => {
            println!("Failed to connect to socket at: {}", SERVER_ADDR);
            std::process::exit(1)
        }
    };
    // prevent io stream operation from blocking sockets in case of slow communication
    server.set_nonblocking(true).expect("Failed to initiate non-blocking!");

    let mut clients = vec![];
    let colour = Arc::new(Mutex::new(0));
    {
        let mut col = colour.lock().unwrap();
        *col = 1;
    }

    // create channel for communication between threads
    let (sender, receiver) = mpsc::channel::<String>();

    loop {
        /* Start listening thread on new connecting client. */
        if let Ok((mut socket, addr)) = server.accept() {

            println!("Client {} connected.", addr);

            let _sender = sender.clone();

            let client = socket.try_clone().expect("Failed to clone client! Client wont receive messages!");

            clients.push(client);
            
            let c;
            match clients.len() % 2 {
                0 => {
                    c = 1;
                },
                1 => {
                    c = 2;
                },
                _ => c = 0
            };

            let colour = Arc::clone(&colour);

            thread::spawn(move || loop {

                let mut msg_buff = vec![0; MSG_SIZE];

                /* Read and relay message from client. */
                match socket.read_exact(&mut msg_buff) {
                    // received message
                    Ok(_) => {
                        let _msg = msg_buff
                            .into_iter()
                            .take_while(|&x| x != 0)
                            .collect::<Vec<_>>();
                        let msg = String::from_utf8(_msg).expect("Invalid UTF-8 message!");

                        let col = colour.lock().unwrap();
                        if c == *col {
                            println!("{}: {:?}", addr, msg);
                            _sender.send(msg).expect("Failed to relay message!")
                        }
                    }, 
                    // no message in stream
                    Err(ref err) if err.kind() == ErrorKind::WouldBlock => (),
                    // connection error
                    Err(_) => {
                        println!("Closing connection with: {}", addr);
                        break;
                    }
                }

                sleep();
            });
        }

        /* Broadcast incoming messages. */
        if let Ok(msg) = receiver.try_recv() {
            // send message to all clients
            clients = clients.into_iter().filter_map(|mut client| {
                let mut msg_buff = msg.clone().into_bytes();
                // add zero character to mark end of message
                msg_buff.resize(MSG_SIZE, 0);

                client.write_all(&msg_buff).map(|_| client).ok()
            }).collect::<Vec<_>>();
            let mut col = colour.lock().unwrap();
            *col = match *col {
                1 => 2,
                2 => 1,
                _ => 1
            }
        }

        sleep();
    }
}