use std::io::{BufRead, BufReader, BufWriter};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::str;
use std::thread;
use std::time::Duration;
use std::vec::Vec;

use http_muncher::{Parser, ParserHandler};

const BUF_SIZE: usize = 8192;

struct CuckooHttpHandler;

impl ParserHandler for CuckooHttpHandler {
    fn on_header_field(&mut self, parser: &mut Parser, header: &[u8]) -> bool {
        println!("{}: ", str::from_utf8(header).unwrap());

        true
    }

    fn on_header_value(&mut self, parser: &mut Parser, value: &[u8]) -> bool {
        println!("\t{}", str::from_utf8(value).unwrap());

        true
    }
}

fn handle_client(mut client_stream: TcpStream) {
    client_stream
        .set_read_timeout(Some(Duration::new(20, 0)))
        .unwrap();
    client_stream
        .set_write_timeout(Some(Duration::new(5, 0)))
        .unwrap();

    let mut buffered = BufReader::new(&client_stream);

    let mut callbacks_handler = CuckooHttpHandler {};
    let mut parser = Parser::request();

    let mut current_buf = Vec::new();

    loop {
        let n = buffered.read_until(b'\n', &mut current_buf);

        // If for some reason we can't read, then close the
        // connection.
        if n.is_err() {
            client_stream.shutdown(Shutdown::Both).unwrap();
            return;
        }

        let sz = n.unwrap();

        parser.parse(&mut callbacks_handler, &current_buf);
    }
}

fn server_thread(local_ip: String) {
    let listener = TcpListener::bind(local_ip.clone()).unwrap();
    println!("Started listening on {}", &local_ip);
    for stream in listener.incoming() {
        if stream.is_err() {
            continue;
        }
        thread::spawn(|| handle_client(stream.unwrap()));
    }
}
