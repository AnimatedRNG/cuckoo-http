extern crate cuckoo_http;

use cuckoo_http::http_server;

fn main() {
    http_server::server_start("0.0.0.0:8080".to_string());
}
