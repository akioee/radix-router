use radix_router::{Meta, Router};
use std::{
    io::{BufRead, BufReader, Result},
    net::{TcpListener, TcpStream},
    sync::{Arc, Mutex},
};

/// Simple http example, it's bad to use `Mutex` in real world XD.
fn main() -> Result<()> {
    let router = Arc::new(Mutex::new(create_router()));

    for stream in TcpListener::bind("127.0.0.1:8889")?.incoming() {
        match stream {
            Ok(mut s) => {
                let router_cloned = router.clone();
                std::thread::spawn(move || http_handler(&mut s, router_cloned)).join().expect("Error");
            }
            Err(_) => panic!("Oops~"),
        }
    }

    Ok(())
}

fn create_router() -> Router {
    let mut router = Router::new();

    router.insert("/a/:name", Meta::default());
    router
}

fn http_handler(stream: &mut TcpStream, router: Arc<Mutex<Router>>) {
    let router_locked = router.lock().expect("Error get lock");

    if let Some(path) = extract_url_from_stream(stream) {
        println!("{:?}", router_locked.lookup(dbg!(&path)));
    }
}

fn extract_url_from_stream(stream: &mut TcpStream) -> Option<String> {
    let mut reader = BufReader::new(stream);
    let mut request_line = String::new();

    if reader.read_line(&mut request_line).is_err() {
        return None;
    }

    let parts = request_line.split_whitespace().collect::<Vec<_>>();

    if parts.len() < 2 {
        return None;
    }

    Some(parts[1].to_owned())
}
