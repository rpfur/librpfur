// Enable Macros 1.1 to be used by serde_macros
#![feature(proc_macro)]

// Disable warnings for code that will be used later on
#![allow(dead_code)]
#![allow(unused_imports)]

#[macro_use] extern crate log;
#[macro_use] extern crate rand;
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

/// Number of characters in the authorization token
const TOKEN_LENGTH: usize = 32;

use std::io;
use rand::{Rng, OsRng};
use std::sync::{Arc, Mutex};
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
struct Token([char; TOKEN_LENGTH]);
impl Token{
    /// Fills the contents of this token using a given Rng
    fn _fill_by_rng<R: Rng>(&mut self, rng: &mut R){
        let mut i = 0;
        for character in rng.gen_ascii_chars().filter(|character| !character.is_uppercase()).take(TOKEN_LENGTH){
            if i <= TOKEN_LENGTH{
                self.0[i] = character;
                i += 1;
            }else{
                return
            }
        }
    }

    /// Try to retrieve a cached instance of OsRng through the use of a singleton
    ///
    /// Deprecated: This funcion is most likely not really needed,
    /// as creating a new OsRng for every generation secure, fast enough,
    /// and simpler, however, note that, proven otherwise, this function
    /// will return to active usage.
    #[deprecated]
    fn _rng() -> Option<Arc<Mutex<OsRng>>>{
        use std::sync::{Once, ONCE_INIT};
        static mut RNG: *const Arc<Mutex<OsRng>> = 0 as *const Arc<Mutex<OsRng>>;
        static mut ONCE: Once = ONCE_INIT;
        unsafe{
            // Try creating a new OsRng once, and,
            // in case it is successful, cache it for later calls
            ONCE.call_once(||{
                use std::mem;
                if let Ok(rng) = OsRng::new(){
                    RNG = mem::transmute(Box::new(Arc::new(Mutex::new(rng))));
                }
            });

            // Return a copy of the Arc if it was generated successfuly
            match RNG.is_null(){
                false => Some((*RNG).clone()),
                true  => None
            }
        }
    }

    pub fn generate() -> Result<Token, io::Error>{
        // Create an Token to be filled later (using _fill_by_rng())
        let mut token = Token([' '; TOKEN_LENGTH]);

        // Try to use OsRng to generate the sequence, as recommended by the `rand`
        // documentation, for the native algorythms that generate random numbers are specialized
        // to generate number suitable for an encryption algorythm.
        match OsRng::new(){
            Ok(mut rng) => {
                use std::ops::DerefMut;
                token._fill_by_rng(&mut rng)
            },
            Err(what) => {
                // We're now entering the Bone Zone!
                // Reffer to Cargo.toml and the allow-unsafe-tokens feature for more
                // information on why this is a terrible idea when we're talking about user safety.
                if cfg!(feature = "allow-unsafe-tokens"){
                    use rand::thread_rng;
                    token._fill_by_rng(&mut thread_rng())
                }else{
                    // For real use cases it's better for the generation to fail
                    // than to generate something that will end up being unsafe.
                    return Err(what)
                }
            }
        }

        // Return the now filled token
        Ok(token)
    }
}
use std::fmt;
impl fmt::Display for Token{
    fn fmt(&self, target: &mut fmt::Formatter) -> fmt::Result{
        use std::iter::FromIterator;
        write!(target, "{}", self.0.iter().cloned().collect::<String>())
    }
}

#[derive(Serialize, Deserialize, Debug)]
enum Response{
    Connected{ server: String },
    LoginSuccesful{

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
    fn token(){
        // Number of threads to be spawn and tokens to be generated in every thread
        const THREAD_COUNT: usize = 512;
        const RESULT_COUNT: usize = 10;

        // Generate a number of tokens simultaniously across different threads
        let mut threads = Vec::new();
        for i in 0..THREAD_COUNT{
            use std::thread;
            threads.push(thread::spawn(move || {
                use super::Token;

                let mut tokens = Vec::new();
                for i in 0..RESULT_COUNT{
                    tokens.push(Token::generate().unwrap())
                }
                tokens
            }));

            // Sleep a little
            use std::time::Duration;
            thread::sleep(Duration::new(0, 5000));
        }

        // Collect all the generated tokens
        use std::iter;
        let tokens = threads.into_iter()
            .flat_map(|thread| thread.join().unwrap().into_iter())
            .collect::<Vec<_>>();

        // Make sure no pair of tokens match
        let reference = tokens.clone();
        let mut counter: usize = 0;
        for token in tokens.iter(){
            counter += 1;
            for comparator in reference.iter().skip(counter){
                assert!(token != comparator);
            }
        }
    }

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
