#![recursion_limit = "128"]

#[macro_use]
extern crate stdweb;

extern crate cuckoo_http;

use stdweb::web::{document, INode};

use cuckoo_http::cuckoo;
use cuckoo_http::simple_miner;

fn main() {
    stdweb::initialize();

    js! { alert("Starting execution..."); }

    let nl = document().head().unwrap().as_node().child_nodes();
    let header = nl.item(3).unwrap().text_content().unwrap();
    let easipct = nl.item(5)
        .unwrap()
        .text_content()
        .unwrap()
        .parse::<i64>()
        .unwrap();
    let difficulty = nl.item(7)
        .unwrap()
        .text_content()
        .unwrap()
        .parse::<f64>()
        .unwrap();
    let msg = nl.item(9).unwrap().text_content().unwrap();

    let graph_v = cuckoo::hash_header(header.as_bytes());

    let easiness: i32 = ((easipct * cuckoo::NNODES as i64) / 100) as i32;
    let hash_difficulty: u64 = ((difficulty / 100.0) * std::u64::MAX as f64) as u64;
    let a = simple_miner::solve(simple_miner::CuckooSolve {
        graph_v: graph_v,
        easiness: easiness,
        hash_difficulty: hash_difficulty,
        cuckoo: vec![0; (1 + cuckoo::NNODES) as usize],
    });

    let message = a.unwrap()
        .into_iter()
        .map(|x| format!("{:x} ", x))
        .collect::<Vec<_>>()
        .concat();

    js! {
        alert( @{message.clone()} );
    }

    js! {
        var xhr = new XMLHttpRequest();
        xhr.open("GET", '/', true);
        xhr.setRequestHeader("X-Cuckoo-Header", @{header});
        xhr.setRequestHeader("X-Cuckoo-Solution", @{message.trim()});
        xhr.setRequestHeader("X-Cuckoo-Original-Request", @{msg});

        xhr.onreadystatechange = function () {
            if (xhr.readyState == XMLHttpRequest.DONE && xhr.status == 200) {
                alert("Got response!");
                document.open();
                document.write(this.responseText);
                document.close();
            }
        };

        xhr.send("");
    }

    stdweb::event_loop();
}
