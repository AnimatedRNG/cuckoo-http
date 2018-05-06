use std::io;
use std::io::{BufRead, BufReader, BufWriter};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::str;
use std::thread;
use std::time::Duration;
use std::vec::Vec;

const BUF_SIZE: usize = 8192;
const CONTENT_LENGTH: &[u8] = b"Content-Length:";
const HEADER_END: &[u8] = b"\n\r\n";

struct TCPRead {
    tcp_stream: TcpStream,
    buf: [u8; BUF_SIZE],
    buf_ptr: usize,
    end_of_read: usize,
    offset: usize,
}

impl TCPRead {
    #[inline]
    fn get(&mut self) -> Option<u8> {
        if self.buf_ptr == self.end_of_read {
            match self.tcp_stream.read(&mut self.buf) {
                Err(_) => {
                    self.tcp_stream.shutdown(Shutdown::Both).unwrap();
                    return None;
                }
                Ok(n) => {
                    self.buf_ptr = 0;
                    self.offset += self.end_of_read;
                    self.end_of_read = n;
                }
            };
        }

        let c = self.buf[self.buf_ptr];
        //println!("Did TCP read, read {:?} at {:?}", c as char, self.buf_ptr);
        self.buf_ptr += 1;
        Some(c)
    }

    fn ptr(&self) -> usize {
        self.offset + self.buf_ptr
    }
}

enum ContentLengthState {
    READING_NAME,
    READING_WHITESPACE,
    READING_NUMBER,
}

enum HTTPReadState {
    READING_CONTENT_LENGTH,
    READING_UNTIL_BODY,
    READING_BODY,
}

struct HTTPRead {
    tcp_read: TCPRead,
    max_len: usize,

    content_length_ptr: usize,
    content_length: Option<usize>,
    tmp_content_length_buffer: Vec<u8>,
    content_length_state: ContentLengthState,

    read_state: HTTPReadState,

    rest_of_http_header: usize,

    end_ptr: usize,
}

impl<'a> HTTPRead {
    fn new(tcp_stream: TcpStream, max_len: usize) -> HTTPRead {
        HTTPRead {
            tcp_read: TCPRead {
                tcp_stream: tcp_stream,
                buf: [0; BUF_SIZE],
                buf_ptr: 0,
                end_of_read: 0,
                offset: 0,
            },
            max_len: max_len,

            content_length_ptr: 0,
            content_length: None,
            tmp_content_length_buffer: Vec::new(),
            content_length_state: ContentLengthState::READING_NAME,

            read_state: HTTPReadState::READING_CONTENT_LENGTH,

            rest_of_http_header: 0,

            end_ptr: 0,
        }
    }

    fn write(&mut self, data: &[u8]) -> Result<(), io::Error> {
        self.tcp_read.tcp_stream.write(data)?;
        self.tcp_read.tcp_stream.flush()?;
        Ok(())
    }

    fn close(&mut self) {
        self.tcp_read.tcp_stream.shutdown(Shutdown::Both).unwrap();
    }

    fn read_until_body(&mut self, c: &u8) -> bool {
        if *c == HEADER_END[self.rest_of_http_header] {
            self.rest_of_http_header += 1;
        } else {
            self.rest_of_http_header = 0;
        }

        if self.rest_of_http_header == HEADER_END.len() {
            self.rest_of_http_header = 0;
            println!("Found body!");
            return true;
        } else {
            return false;
        }
    }

    fn read_content_header(&mut self, c: &u8) {
        self.content_length_state = match self.content_length_state {
            ContentLengthState::READING_NAME => {
                if self.content_length_ptr >= CONTENT_LENGTH.len() {
                    self.content_length_ptr = 0;
                    ContentLengthState::READING_WHITESPACE
                } else {
                    ContentLengthState::READING_NAME
                }
            }
            ContentLengthState::READING_WHITESPACE => {
                if *c != b' ' && *c != b'\t' {
                    ContentLengthState::READING_NUMBER
                } else {
                    ContentLengthState::READING_WHITESPACE
                }
            }
            ContentLengthState::READING_NUMBER => {
                if *c == b'\r' {
                    let mut accum: usize = 0;
                    for ci in &self.tmp_content_length_buffer {
                        if *ci >= b'0' && *ci <= b'9' {
                            accum = ((ci - b'0') as usize) + accum * 10;
                        }
                    }

                    self.tmp_content_length_buffer.clear();
                    self.content_length = Some(accum);
                    self.content_length_ptr = 0;
                }
                ContentLengthState::READING_NUMBER
            }
        };

        match self.content_length_state {
            ContentLengthState::READING_NAME => {
                if self.content_length_ptr < CONTENT_LENGTH.len() {
                    if *c == CONTENT_LENGTH[self.content_length_ptr] {
                        self.content_length_ptr += 1;
                    } else {
                        self.content_length_ptr = 0;
                    }
                }
            }
            ContentLengthState::READING_WHITESPACE => {
                //if *c == b' ' || *c == b'\t' {
                //self.content_length_ptr += 1;
                //}
            }
            ContentLengthState::READING_NUMBER => {
                if *c != b'\r' {
                    self.tmp_content_length_buffer.push(*c);
                }
            }
        };
    }

    fn next(&mut self) -> Option<String> {
        let mut result: Vec<u8> = Vec::new();

        self.content_length_ptr = 0;
        self.rest_of_http_header = 0;
        self.end_ptr = 0;
        self.read_state = HTTPReadState::READING_CONTENT_LENGTH;

        let mut i = self.tcp_read.ptr();
        loop {
            let c_raw = self.tcp_read.get();
            if c_raw == None {
                return None;
            }
            let c = c_raw.unwrap();
            result.push(c);

            i += 1;

            println!("{:?}", str::from_utf8(&result).unwrap());

            match self.read_state {
                HTTPReadState::READING_CONTENT_LENGTH => {
                    self.read_content_header(&c);
                    if self.content_length.is_some() {
                        self.read_state = HTTPReadState::READING_UNTIL_BODY;
                    }
                }
                HTTPReadState::READING_UNTIL_BODY => {
                    let cl = self.content_length.unwrap();
                    println!("Content length: {:?}", cl);
                    if self.read_until_body(&c) {
                        self.end_ptr = i + cl;
                        self.read_state = HTTPReadState::READING_BODY;
                    }
                }
                HTTPReadState::READING_BODY => {
                    //println!("end_ptr is {:?}, i is {:?}", i, self.end_ptr);
                    if i >= self.end_ptr - 1 {
                        println!("Done!");
                        break;
                    }
                }
            };
        }

        return str::from_utf8(&result)
            .and_then(|s: &str| Ok(s.to_string()))
            .ok();
    }
}

enum VerifyStatus {
    UNVERIFIED,
    INVALID,
    VALID,
}

fn requires_cuckoo(_: &String) -> bool {
    true
}

fn verified(_: &String) -> VerifyStatus {
    VerifyStatus::UNVERIFIED
}

fn handle_client(mut client_stream: TcpStream) {
    client_stream
        .set_read_timeout(Some(Duration::new(20, 0)))
        .unwrap();
    client_stream
        .set_write_timeout(Some(Duration::new(5, 0)))
        .unwrap();

    let mut h = HTTPRead::new(client_stream, BUF_SIZE);

    loop {
        let msg_raw = h.next();
        if msg_raw.is_none() {
            return;
        }
        let msg = msg_raw.unwrap();

        println!("{:?}", msg);

        match verified(&msg) {
            VerifyStatus::UNVERIFIED => {
                if requires_cuckoo(&msg) {
                    // Reply with request details
                    let body = format!("<p>{}</p>", msg);
                    let http_message = format!("HTTP/1.1 200 OK\r\nCache-Control: no-cache, private\r\nContent-Length: {}\r\nContent-Type: text-html\r\nConnection: close\r\n\r\n{}", body.len(), body);
                    if h.write(http_message.as_bytes()).is_err() {
                        h.close();
                    }

                // Then drop the connection
                } else {
                    // Forward it to the server
                }
            }
            VerifyStatus::INVALID => {
                // Drop the connection
            }
            VerifyStatus::VALID => {
                // Forward sub-message to the server
            }
        }
    }
}

pub fn server_start(local_ip: String) {
    let listener = TcpListener::bind(local_ip.clone()).unwrap();
    for stream in listener.incoming() {
        if stream.is_err() {
            continue;
        }
        thread::spawn(|| handle_client(stream.unwrap()));
    }
}

#[cfg(test)]
mod tests {
    use http_server::server_start;
    use std::io::Write;
    use std::net::TcpStream;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;
    use std::vec::Vec;

    fn set_up_connection() -> mpsc::Sender<Vec<u8>> {
        thread::spawn(|| server_start("127.0.0.1:8080".to_string()));
        let (tx, rx): (mpsc::Sender<Vec<u8>>, mpsc::Receiver<Vec<u8>>) = mpsc::channel();
        thread::spawn(move || {
            let mut s = TcpStream::connect("127.0.0.1:8080").unwrap();
            for a in rx.iter() {
                s.write(&a).unwrap();
                s.flush().unwrap();
            }
        });
        tx
    }

    #[test]
    fn standard_read_works() {
        let tx = set_up_connection();
        tx.send(b"GET / HTTP/1.1\r\nContent-Length: 10\r\n\r\n012345789".to_vec())
            .unwrap();

        thread::sleep(Duration::new(3, 0));
    }
}
