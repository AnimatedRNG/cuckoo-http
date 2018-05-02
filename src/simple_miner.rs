const MAXPATHLEN: usize = 4096;

use std::cmp::min as _min;
use std::collections::HashSet;

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
    us: [i32; MAXPATHLEN],
    mut nu: i32,
    vs: [i32; MAXPATHLEN],
    mut nv: i32,
) -> [i32; PROOFSIZE] {
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
    if n != PROOFSIZE {
        println!("Only recovered {:?} nonces", n)
    }
    return new_proof;
}

pub fn solve(mut cs: CuckooSolve) -> Option<[i32; PROOFSIZE]> {
    let mut us: [i32; MAXPATHLEN] = [0; MAXPATHLEN];
    let mut vs: [i32; MAXPATHLEN] = [0; MAXPATHLEN];
    for nonce in 0..cs.easiness {
        us[0] = sipnode(cs.graph_v, nonce, 0);
        vs[0] = NEDGES + sipnode(cs.graph_v, nonce, 1);

        let u = cs.cuckoo[us[0] as usize];
        let v = cs.cuckoo[vs[0] as usize];

        if u == vs[0] || v == us[0] {
            continue;
        }

        let nu_raw = path(&cs, u, &mut us);
        let nv_raw = path(&cs, v, &mut vs);

        if nu_raw.is_none() || nv_raw.is_none() {
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
                return Some(solution(&cs, us, nu, vs, nv));
            }

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
    }
    return None;
}
