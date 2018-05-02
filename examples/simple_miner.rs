extern crate cuckoo_http;

use std::fs;

use cuckoo_http::cuckoo;
use cuckoo_http::cuckoo::NNODES;
use cuckoo_http::simple_miner::{solve, CuckooSolve};

fn main() {
    let mut header = String::new();
    let mut easipct: i32 = 70;

    let mut args = std::env::args();

    args.next();
    assert!(args.len() >= 2);

    let filename: String = args.next().unwrap();

    loop {
        match args.next() {
            Some(arg) => {
                if arg == "-e" {
                    easipct = args.next().unwrap().parse::<i32>().unwrap();
                } else if arg == "-h" {
                    header = args.next().unwrap();
                }
            }
            None => break,
        }
    }

    let easiness: i32 = ((easipct as i64 * NNODES as i64) / 100) as i32;
    let v = cuckoo::hash_header(header.as_bytes());
    /*let v: [u64; 4] = [
        1449310910991872227,
        2646268962349054874,
        5517924826087534119,
        6176777564751238564,
    ];*/

    let cs = CuckooSolve {
        graph_v: v,
        easiness: easiness,
        cuckoo: vec![0; (1 + NNODES) as usize],
    };

    let result = solve(cs);

    match result {
        None => return,
        Some(r) => fs::write(
            filename,
            r.into_iter()
                .map(|x| format!("{:x} ", x))
                .collect::<Vec<_>>()
                .concat(),
        ).unwrap(),
    };
}
