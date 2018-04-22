const MAXPATHLEN: usize = 4096;

use std::collections::HashSet;

use cuckoo::NNODES;
use cuckoo::NEDGES;
use cuckoo::PROOFSIZE;
use cuckoo::Edge;
use cuckoo::sipedge;

struct CuckooSolve<'a> {
    graph_v: [i64; 4],
    easiness: i32,
    cuckoo: [usize; 1 + NNODES],
    sols: &'a [[i32; PROOFSIZE]],
    nsols: usize,
    nthreads: usize,
}

// Refactor sometime
fn path(v: CuckooSolve, mut u: usize, us: &mut [usize; 1 + NNODES]) -> Option<usize> {
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
        u = v.cuckoo[u];
    }

    return Some(nu);
}

fn solution(
    v: CuckooSolve,
    us: &mut [usize; 1 + NNODES],
    nu: usize,
    vs: [usize; MAXPATHLEN],
    nv: usize,
) {
    let cycle: HashSet<Edge> = HashSet::new();
    let mut n = 0;

    cycle.insert(Edge {
        u: us[0] as i32,
        v: (vs[0] - NEDGES) as i32,
    });
    while nu != 0 {
        nu -= 1;
        cycle.insert(Edge {
            u: us[(nu + 1) & !1],
            v: us[nu | 1] - NEDGES,
        });
    }
    while nv != 0 {
        nv -= 1;
        cycle.insert(Edge {
            u: vs[nv | 1],
            v: vs[(nv + 1) & !1] - NEDGES,
        });
    }

    n = 0;
    for nonce in 0..v.easiness {
        let e = sipedge(v, nonce);
        if cycle.contains(e) {
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

fn solve(v: CuckooSolve, id: i32) {}
