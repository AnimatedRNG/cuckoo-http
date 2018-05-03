use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::thread;
use std::time::Duration;
use std::vec::Vec;

const BUF_SIZE: usize = 8192;

fn safe_shutdown(mut client_stream: TcpStream) {
    match client_stream.shutdown(Shutdown::Both) {
        Err(_) => {}
        Ok(_) => {}
    }
}

fn handle_client(mut client_stream: TcpStream) {
    client_stream.set_read_timeout(Some(Duration::new(20, 0)));
    client_stream.set_write_timeout(Some(Duration::new(5, 0)));

    let mut req = Vec::new();
    req.reserve(1024);
    let mut current_buf = [0 as u8; BUF_SIZE];

    let mut start_of_http_request = true;
    let mut found_method_whitespace = false;

    let mut unverified = true;
    let mut content_length: Option<usize> = None;
    let mut in_body = false;

    let mut a: usize = 0;

    loop {
        let n = client_stream.read(&mut current_buf);

        // If for some reason we can't read, then close the
        // connection.
        if n.is_err() {
            safe_shutdown(client_stream);
            return;
        }

        let sz = n.unwrap();

        // We expect the HTTP headers to start with...
        if start_of_http_request {
            'method_loop: for i in 0..sz {
                let ch = current_buf[i] as u8;
                req.push(ch);
                a = i;
                if ch == b' ' {
                    if req != b"GET " && req != b"HEAD " && req != b"POST " && req != b"PUT "
                        && req != b"DELETE " && req != b"TRACE "
                        && req != b"OPTIONS " && req != b"CONNECT "
                        && req != b"PATCH "
                    {
                        safe_shutdown(client_stream);
                        return;
                    }

                    found_method_whitespace = true;
                    break 'method_loop;
                }
            }

            if found_method_whitespace {}
        }
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
