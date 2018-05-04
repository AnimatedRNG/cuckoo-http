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
        self.buf_ptr += 1;
        Some(c)
    }

    fn ptr(&self) -> usize {
        self.offset + self.buf_ptr
    }
}

struct HTTPRead {
    tcp_read: TCPRead,
    max_len: usize,

    content_length_ptr: usize,
    content_length: Option<usize>,
    in_content_length: bool,
    tmp_content_length_buffer: Vec<u8>,

    reached_content_length_but_not_body: bool,
    rest_of_http_header: usize,
    in_body: bool,

    end_ptr: usize,
}

impl HTTPRead {
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
            in_content_length: false,
            tmp_content_length_buffer: Vec::new(),

            reached_content_length_but_not_body: false,
            rest_of_http_header: 0,
            in_body: false,

            end_ptr: 0,
        }
    }

    fn read_until_body(&mut self, c: &u8) {
        if self.rest_of_http_header < HEADER_END.len() {
            if *c == HEADER_END[self.rest_of_http_header] {
                self.rest_of_http_header += 1;
            } else {
                self.rest_of_http_header = 0;
            }
        } else {
            self.rest_of_http_header = 0;
            self.reached_content_length_but_not_body = false;
        }
    }

    fn read_content_header(&mut self, c: &u8) {
        if self.content_length_ptr < CONTENT_LENGTH.len() {
            if *c == CONTENT_LENGTH[self.content_length_ptr] {
                self.content_length_ptr += 1;
            } else {
                self.content_length_ptr = 0;
            }
        } else {
            if *c == b' ' || *c == b'\t' {
                self.content_length_ptr += 1;
                self.in_content_length = true;
            } else {
                if self.in_content_length {
                    self.tmp_content_length_buffer.push(*c);
                    self.in_content_length = false;
                } else if *c == b'\r' {
                    let mut accum: usize = 0;
                    for ci in &self.tmp_content_length_buffer {
                        if *ci >= b'0' && *ci <= b'9' {
                            accum += ((ci - b'0') as usize) + accum * 10;
                        }
                    }
                    self.tmp_content_length_buffer.clear();
                    self.content_length = Some(accum);
                    self.reached_content_length_but_not_body = true;
                    self.content_length_ptr = 0;
                }
            }
        }
    }
}

impl Iterator for HTTPRead {
    type Item = String;

    fn next(&mut self) -> Option<String> {
        let mut result: Vec<u8> = Vec::new();

        self.content_length_ptr = 0;
        self.rest_of_http_header = 0;
        self.end_ptr = 0;

        let mut i = self.tcp_read.ptr();
        loop {
            let c_raw = self.tcp_read.get();
            if c_raw == None {
                return None;
            }
            let c = c_raw.unwrap();
            result.push(c);

            if self.content_length.is_none() {
                self.read_content_header(&c);
            } else {
                let cl = self.content_length.unwrap();
                if self.reached_content_length_but_not_body {
                    self.read_until_body(&c);
                } else {
                    if self.in_body {
                        if i >= self.end_ptr {
                            break;
                        }
                    } else {
                        self.end_ptr = i + cl;
                    }
                }
            }

            i += 1;
        }

        return None;
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
