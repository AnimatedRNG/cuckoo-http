use std::io;
use std::io::{BufRead, BufReader, BufWriter};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpListener, TcpStream};
use std::str;
use std::fs;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use std::vec::Vec;

const BUF_SIZE: usize = 8192;
const CONTENT_LENGTH: &[u8] = b"Content-Length:";
const HEADER_END: &[u8] = b"\n\r\n";
const HEADER_LENGTH: usize = 32;

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
    READING_METHOD,
    READING_URL,
    READING_CONTENT_LENGTH,
    READING_UNTIL_BODY,
    READING_BODY,
}

struct HTTPRead {
    tcp_read: TCPRead,
    max_len: usize,

    tmp_url_buffer: Vec<u8>,

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

            tmp_url_buffer: Vec::new(),

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
            },
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

    fn next(&mut self) -> Option<(String, String)> {
        let mut result: Vec<u8> = Vec::new();

        self.content_length_ptr = 0;
        self.rest_of_http_header = 0;
        self.end_ptr = 0;
        self.read_state = HTTPReadState::READING_METHOD;

        let mut i = self.tcp_read.ptr();
        loop {
            let c_raw = self.tcp_read.get();
            if c_raw == None {
                return None;
            }
            let c = c_raw.unwrap();
            result.push(c);

            i += 1;

            match self.read_state {
                HTTPReadState::READING_METHOD => {
                    if c == b' ' || c == b'\t' {
                        if result == b"GET " {
                            self.content_length = Some(0);
                        }
                        self.read_state = HTTPReadState::READING_URL;
                    }
                }
                HTTPReadState::READING_URL => {
                    if c == b' ' || c == b'\t' {
                        self.read_state = HTTPReadState::READING_CONTENT_LENGTH;
                    } else {
                        self.tmp_url_buffer.push(c);
                    }
                }
                HTTPReadState::READING_CONTENT_LENGTH => {
                    self.read_content_header(&c);
                    if self.content_length.is_some() {
                        self.read_state = HTTPReadState::READING_UNTIL_BODY;
                    }
                }
                HTTPReadState::READING_UNTIL_BODY => {
                    let cl = self.content_length.unwrap();
                    if self.read_until_body(&c) {
                        if cl == 0 {
                            println!("Done!");
                            self.read_state = HTTPReadState::READING_METHOD;
                            break;
                        }

                        self.end_ptr = i + cl;
                        self.read_state = HTTPReadState::READING_BODY;
                    }
                }
                HTTPReadState::READING_BODY => {
                    //println!("end_ptr is {:?}, i is {:?}", i, self.end_ptr);
                    if i >= self.end_ptr - 1 {
                        println!("Done!");
                        self.read_state = HTTPReadState::READING_METHOD;
                        break;
                    }
                }
            };
        }

        let str_result = match str::from_utf8(&result) {
            Err(_) => None,
            Ok(s) => Some(s.to_string()),
        };

        let url_result = match str::from_utf8(&self.tmp_url_buffer) {
            Err(_) => None,
            Ok(s) => Some(s.to_string()),
        };

        if str_result.is_some() && url_result.is_some() {
            Some((str_result.unwrap(), url_result.unwrap()))
        } else {
            None
        }
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

fn format_response_text(body: &String, content_type: &'static str) -> String {
    return format!("HTTP/1.1 200 OK\r\nCache-Control: no-cache, private\r\nContent-Length: {}\r\nContent-Type: {}\r\nConnection: close\r\n\r\n{}", body.len(), content_type, body);
}

fn format_response_binary(mut body: Vec<u8>, content_type: &'static str) -> Vec<u8> {
    let mut header = format!("HTTP/1.1 200 OK\r\nCache-Control: no-cache, private\r\nContent-Length: {}\r\nContent-Type: {}\r\nConnection: close\r\n\r\n", body.len(), content_type).as_bytes().to_vec();
    header.append(&mut body);
    return header;
}

fn efficient_replace(orig_text: &[u8], text_to_find: &[u8], replace_with: &[u8]) -> Vec<u8>{
    let len = orig_text.len();
    let mut a = 0;

    let mut new_text = Vec::new();

    let mut appending: bool = false;

    for i in 0..len {
        let c = orig_text[i];
        new_text.push(c);

        if !appending {
            if c == text_to_find[a] {
                a += 1;
            } else {
                a = 0;
            }

            if a == text_to_find.len() {
                let end_ptr = i + 1;
                let start_ptr = end_ptr - a;

                new_text.truncate(start_ptr);
                for c1 in replace_with {
                    new_text.push(*c1);
                }

                appending = true;
            }
        }
    }

    return new_text;
}

#[derive(Hash, PartialEq, Eq)]
enum StaticResource {
    WEB_MINER_JS,
    WEB_MINER_WASM,
    WEB_MINER_HTML,
}

struct CuckooProblem {
    easipct: i32,
    difficulty: f64,
}

type CuckooMap = HashMap<[u8; HEADER_LENGTH], CuckooProblem>;

fn handle_client(mut client_stream: TcpStream,
cached_files: HashMap<StaticResource, Vec<u8>>) {
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
        let (msg, url) = msg_raw.unwrap();

        println!("{}", msg);
        println!("URL: {}", url);

        if url == "/web_miner.wasm" {
            // TODO: Take this conversion out of HTTP request handling...
            let m = cached_files.get(&StaticResource::WEB_MINER_WASM).unwrap();

            if h.write(m).is_err() {
                h.close();
            } else {
                h.close();
            }

            return;
        } else if url == "/web_miner.js" {
            let m = cached_files.get(&StaticResource::WEB_MINER_JS).unwrap();

            if h.write(m).is_err() {
                h.close();
            } else {
                h.close();
            }

            return;
        }

        match verified(&msg) {
            VerifyStatus::UNVERIFIED => {
                if requires_cuckoo(&url) {
                    // Reply with request details
                    let index = cached_files.get(&StaticResource::WEB_MINER_HTML).unwrap();
                    let header_replaced = efficient_replace(index, b"HEADER", b"test");
                    let easiness_replaced = efficient_replace(&header_replaced, b"EASINESS", b"70");
                    let difficulty_replaced = efficient_replace(&easiness_replaced, b"DIFFICULTY", b"99.9");
                    let m = format_response_binary(difficulty_replaced, "text/html");
                    if h.write(&m).is_err() {
                        h.close();
                    } else {
                        // Then drop the connection
                        h.close();
                    }

                    return;
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

        let mut st = HashMap::new();
        /*st.insert(StaticResource::WEB_MINER_HTML,
                  format_response_text(&mut fs::read_to_string(
                      "static/index.html").unwrap(),
        "text/html").as_bytes().to_vec());*/
        st.insert(StaticResource::WEB_MINER_HTML,
                  fs::read("static/index.html").unwrap());
        st.insert(StaticResource::WEB_MINER_JS,
                  format_response_text(&mut fs::read_to_string(
                      "target/wasm32-unknown-unknown/release/web_miner.js").unwrap(),
                                       "application/javascript").as_bytes().to_vec());
        st.insert(StaticResource::WEB_MINER_WASM,
                  format_response_binary(fs::read(
                      "target/wasm32-unknown-unknown/release/web_miner.wasm").unwrap(),
                                         "application/wasm"
                  ));
        thread::spawn(|| handle_client(stream.unwrap(), st));
    }
}

#[cfg(test)]
mod tests {
    use http_server::{server_start, efficient_replace};
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
    fn efficient_replace_works() {
        let a: [u8; 8] = [0, 1, 2, 5, 5, 5, 1, 2];
        let v1: [u8; 3] = [5, 5, 5];
        let v2: [u8; 1] = [0];
        assert_eq!(efficient_replace(&a, &v1, &v2), vec![0, 1, 2, 0, 1, 2]);

        let v3: [u8; 8] = [0, 1, 2, 5, 5, 5, 1, 2];
        let v4: [u8; 0] = [];
        assert_eq!(efficient_replace(&a, &v3, &v4), vec![]);

        let v5: [u8; 3] = [0, 1, 2];
        let v6: [u8; 5] = [0, 1, 2, 3, 4];
        assert_eq!(efficient_replace(&a, &v5, &v6), vec![0, 1, 2, 3, 4, 5, 5, 5, 1, 2]);
    }

    #[test]
    fn get_works() {
        let tx = set_up_connection();
        tx.send(b"GET /test HTTP/1.1\r\nContent-Length: 10\r\n\r\n".to_vec())
            .unwrap();

        thread::sleep(Duration::new(3, 0));
    }

    #[test]
    fn post_works() {
        let tx = set_up_connection();
        tx.send(b"POST / HTTP/1.1\r\nContent-Length: 10\r\n\r\n{fdfafa}".to_vec())
            .unwrap();

        thread::sleep(Duration::new(3, 0));
    }
}
