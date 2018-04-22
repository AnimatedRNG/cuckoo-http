const MAXPATHLEN: usize = 4096;

use std::collections::HashSet;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::mpsc;
use std::thread;

use cuckoo::NNODES;
use cuckoo::NEDGES;
use cuckoo::PROOFSIZE;
use cuckoo::Edge;
use cuckoo::sipedge;

type Proof = [i32; PROOFSIZE];

struct CuckooSolve<'a> {
    graph_v: [u64; 4],
    easiness: i32,
    cuckoo: [usize; (1 + NNODES) as usize],
    sols: &'a [Proof],
    nsols: usize,
    nthreads: usize,
}

// Refactor sometime
fn path(v: CuckooSolve, mut u: i32, us: &mut [i32; (1 + NNODES) as usize]) -> Option<usize> {
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
        u = v.cuckoo[u as usize] as i32;
    }

    return Some(nu);
}

fn solution(
    v: CuckooSolve,
    tx: &Sender<Proof>,
    us: &mut [i32; (1 + NNODES) as usize],
    mut nu: i32,
    vs: [i32; MAXPATHLEN],
    mut nv: i32,
) {
    let mut cycle: HashSet<Edge> = HashSet::new();
    let mut n = 0;

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

    n = 0;
    for nonce in 0..v.easiness {
        let e = sipedge(v.graph_v, nonce);
        if cycle.contains(&e) {
            v.sols[v.nsols][n] = nonce;
            n += 1;
        }
    }
    if n == PROOFSIZE {
        v.nsols += 1;
    } else {
        println!("Only recovered {:?} nonces", n)
    }
}

fn solve(v: CuckooSolve, id: i32, tx: Sender<Proof>) {}
