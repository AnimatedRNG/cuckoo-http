const MAXPATHLEN: usize = 4096;

use std::cmp::min as _min;
use std::collections::HashSet;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

use cuckoo::Edge;
use cuckoo::NEDGES;
use cuckoo::NNODES;
use cuckoo::PROOFSIZE;
use cuckoo::sipedge;
use cuckoo::sipnode;

type Proof = [i32; PROOFSIZE];

#[derive(Clone)]
pub struct CuckooSolve {
    pub graph_v: [u64; 4],
    pub easiness: i32,
    pub cuckoo: Vec<i32>,
    pub nthreads: usize,
}

// Refactor sometime
fn path(v: &CuckooSolve, mut u: i32, us: &mut [i32; MAXPATHLEN]) -> Option<usize> {
    let mut nu: usize = 0;
    while u != 0 {
        nu += 1;
        if nu >= MAXPATHLEN {
            while nu != 0 && us[nu] != u {
                nu -= 1;
            }
            if nu <= 0 {
                println!("maximum path length exceeded");
            } else {
                println!("illegal {}-cycle", (MAXPATHLEN - nu));
            }
            return Option::None;
        }
        us[nu] = u;
        u = v.cuckoo[u as usize];
    }

    return Some(nu);
}

fn solution(
    v: &CuckooSolve,
    tx: &Sender<Proof>,
    us: [i32; MAXPATHLEN],
    mut nu: i32,
    vs: [i32; MAXPATHLEN],
    mut nv: i32,
) {
    let mut cycle: HashSet<Edge> = HashSet::new();

    cycle.insert(Edge {
        u: us[0] as i32,
        v: (vs[0] - NEDGES) as i32,
    });
    while nu != 0 {
        nu -= 1;
        cycle.insert(Edge {
            u: us[((nu + 1) & !1) as usize],
            v: us[(nu | 1) as usize] - NEDGES,
        });
    }
    while nv != 0 {
        nv -= 1;
        cycle.insert(Edge {
            u: vs[(nv | 1) as usize],
            v: vs[((nv + 1) & !1) as usize] - NEDGES,
        });
    }

    let mut new_proof = [0; PROOFSIZE];
    let mut n = 0;
    for nonce in 0..v.easiness {
        let e = sipedge(v.graph_v, nonce);
        if cycle.contains(&e) {
            new_proof[n] = nonce;
            n += 1;
        }
    }
    if n == PROOFSIZE {
        //tx.send(new_proof).unwrap();
    } else {
        println!("Only recovered {:?} nonces", n)
    }
}

pub fn solve(mut cs: CuckooSolve, id: i32, tx: Sender<Proof>) {
    let mut us: [i32; MAXPATHLEN] = [0; MAXPATHLEN];
    let mut vs: [i32; MAXPATHLEN] = [0; MAXPATHLEN];
    let mut nonce = id;
    while nonce < cs.easiness {
        us[0] = sipnode(cs.graph_v, nonce, 0);
        vs[0] = NEDGES + sipnode(cs.graph_v, nonce, 1);

        let u = cs.cuckoo[us[0] as usize];
        let v = cs.cuckoo[vs[0] as usize];

        if u == vs[0] || v == us[0] {
            nonce += cs.nthreads as i32;
            continue;
        }

        let nu_raw = path(&cs, u, &mut us);
        let nv_raw = path(&cs, v, &mut vs);

        if nu_raw.is_none() || nv_raw.is_none() {
            nonce += cs.nthreads as i32;
            continue;
        }

        let mut nu: i32 = nu_raw.unwrap() as i32;
        let mut nv: i32 = nv_raw.unwrap() as i32;

        if us[nu as usize] == vs[nv as usize] {
            let min = _min(nu, nv);

            nu -= min;
            nv -= min;

            while us[nu as usize] != vs[nv as usize] {
                nu += 1;
                nv += 1;
            }

            let len = nu + nv + 1;
            /*println!(
                "{}-cycle found at {}% for id={}",
                len,
                (nonce * 100) / cs.easiness,
                id
            );*/
            if len == (PROOFSIZE as i32) {
                //solution(&cs, &tx, us, nu, vs, nv);
            }

            nonce += cs.nthreads as i32;
            continue;
        }
        if nu < nv {
            while nu != 0 {
                nu -= 1;
                cs.cuckoo[us[(nu + 1) as usize] as usize] = us[nu as usize];
            }
            cs.cuckoo[us[0] as usize] = vs[0];
        } else {
            while nv != 0 {
                nv -= 1;
                cs.cuckoo[vs[(nv + 1) as usize] as usize] = vs[nv as usize];
            }
            cs.cuckoo[vs[0] as usize] = us[0];
        }

        nonce += cs.nthreads as i32;
    }
}

pub fn run_simple_miner(mut cs: CuckooSolve) {
    let (tx, _) = mpsc::channel();
    let mut join_handles = Vec::new();
    for i in 0..cs.nthreads {
        let new_tx = tx.clone();
        let new_cs = cs.clone();
        join_handles.push(thread::spawn(move || solve(new_cs, i as i32, new_tx)));
    }

    for handle in join_handles {
        handle.join().unwrap();
    }
}
