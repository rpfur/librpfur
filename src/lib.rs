// Enable Macros 1.1 to be used by serde_macros
#![feature(proc_macro)]

// Disable warnings for code that will be used later on
#![allow(dead_code)]
#![allow(unused_imports)]

#[macro_use] extern crate log;
#[macro_use] extern crate sodiumoxide;
#[macro_use] extern crate serde_derive;
#[macro_use] extern crate serde;
#[macro_use] extern crate bincode;
#[macro_use] extern crate json;
#[macro_use] extern crate uuid;

/* Declare submodules */
mod loggers;

use std::net::{TcpStream, TcpListener, IpAddr, SocketAddr};

use sodiumoxide::crypto::box_::PublicKey;
#[derive(Serialize, Deserialize, Debug)]
enum Request{
    /// Registers as a client in the server
    Connect{
        agent: String,
        key: PublicKey,
    },
    Login{
        // TODO: Login request
    }
}

#[derive(Serialize, Deserialize, Debug)]
enum Response{
    Connected{
        server: String,

    }
}

#[derive(Debug)]
struct Fur{
    /// Name of the program working this connection
    agent: String,
    /// IP adress the connection is comming from
    adress: Option<SocketAddr>,
    /// Key needed to encrypt messages targeting this client
    key: PublicKey,
}
use std::thread::JoinHandle;
impl Fur{
    /// Starts a handler thread for a new client
    pub fn handle(mut connection: TcpStream) -> JoinHandle<()>{
        let adress = connection.peer_addr().ok();

        use std::thread;
        thread::spawn(move ||{
            // Read the first request and expect it to be Request::Connect
            use bincode::SizeLimit;
            use bincode::serde::deserialize_from;
            let connection_request: Request = deserialize_from(&mut connection, SizeLimit::Bounded(0x1000))
                .expect(format!("Connection to {:?} failed to be established.", adress).as_str());

            match connection_request{
                Request::Connect{agent, key} => {
                    let fur = Fur{
                        agent:  agent,
                        adress: adress,
                        key:    key,
                    };
                    debug!("Connected to fellow fur: {:?}", fur);
                },
                _ => error!("Connection to {:?} failed dut to lack of a heading request.", adress)
            };
        })
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn connection_listener(){
        // Setup a verbose logger
        use super::loggers;
        loggers::StdLogger::setup_default();

        // Start a basic server on a new thread, which will expect a single connection
        use std::thread;
        let server_thread = thread::spawn(move ||{
            use std::net::TcpListener;
            let mut listener = TcpListener::bind(("localhost", 25365)).unwrap();
            let (stream, _) = listener.accept().unwrap();

            use super::Fur;
            Fur::handle(stream).join();
        });

        // Wait a small ammount of time, which should be enough for the server to start
        use std::time::Duration;
        thread::sleep(Duration::from_millis(50));

        // Connect to it and send a connection request
        use std::net::TcpStream;
        let mut client = TcpStream::connect(("localhost", 25365)).unwrap();

        // Generate a dummy public key
        use sodiumoxide::crypto::box_::gen_keypair;
        let (key, _) = gen_keypair();

        // Create a connection request, serialize it and send it
        use super::Request;
        let request = Request::Connect{
            agent: format!("{}::connection_listener()", module_path!()),
            key: key
        };

        use bincode::SizeLimit;
        use bincode::serde::serialize;
        let buffer = serialize(&request, SizeLimit::Infinite).unwrap();

        use std::io::Write;
        client.write(&buffer);

        // Wait for the server thread to finish
        server_thread.join();
    }
}
