#[macro_use]
extern crate stdweb;

extern crate cuckoo_http;

use std::time;

use cuckoo_http::cuckoo;
use cuckoo_http::simple_miner;

fn main() {
    stdweb::initialize();

    //let start = time::Instant::now();

    let graph_v = cuckoo::hash_header(b"");

    let easipct = 70;
    let difficulty = 99.0;

    let easiness: i32 = ((easipct as i64 * cuckoo::NNODES as i64) / 100) as i32;
    let hash_difficulty: u64 = ((difficulty / 100.0) * std::u64::MAX as f64) as u64;
    let a = simple_miner::solve(simple_miner::CuckooSolve {
        graph_v: graph_v,
        easiness: easiness,
        hash_difficulty: hash_difficulty,
        cuckoo: vec![0; (1 + cuckoo::NNODES) as usize],
    });

    let message = format!(
        "Result: {}",
        a.unwrap()
            .into_iter()
            .fold("".to_string(), |acc, &a| format!("{}{}", acc, a))
);
    /*let elapsed = start.elapsed();
    let sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000_000_000.0);
    let message = format!("Seconds: {}", sec);*/
    js! {
        alert( @{message} );
    }

    stdweb::event_loop();
}
