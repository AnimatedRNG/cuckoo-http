extern crate cuckoo_http;

use std::io::Read;

use cuckoo_http::cuckoo;
use cuckoo_http::cuckoo::NNODES;

fn main() {
    let mut header = String::new();
    let mut easipct: i32 = 70;
    let mut difficulty: f64 = 50.0;

    let mut args = std::env::args();

    assert!(args.len() >= 2);
    args.next();
    let filename: String = args.next().unwrap();

    loop {
        match args.next() {
            Some(arg) => {
                if arg == "-e" {
                    easipct = args.next().unwrap().parse::<i32>().unwrap();
                } else if arg == "-d" {
                    difficulty = (args.next().unwrap().parse::<f64>().unwrap() - 1e-6).abs();
                } else if arg == "-h" {
                    header = args.next().unwrap();
                }
            }
            None => break,
        }
    }

    let mut f = std::fs::File::open(filename).expect("Cannot read nonces");
    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .expect("Unable to read the file");
    contents = contents.trim().to_string();

    let mut raw_nonces: Vec<i32> = contents
        .split(" ")
        .map(|a| i32::from_str_radix(a, 16).unwrap())
        .collect();
    let mut nonces = [0; cuckoo::PROOFSIZE];
    nonces.copy_from_slice(&mut raw_nonces);

    let easiness: i32 = ((easipct as i64 * NNODES as i64) / 100) as i32;
    let hash_difficulty: u64 = ((difficulty / 100.0) * std::u64::MAX as f64) as u64;
    let v = cuckoo::hash_header(header.as_bytes());

    let result = cuckoo::verify(v, nonces, easiness, hash_difficulty);
    if result {
        println!("Verified!");
    } else {
        println!("Failed!");
    }
}
