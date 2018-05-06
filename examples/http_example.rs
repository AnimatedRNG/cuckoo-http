extern crate cuckoo_http;

use cuckoo_http::http_server;

fn main() {
    http_server::server_start("127.0.0.1:8080".to_string());
}
