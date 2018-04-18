extern crate blake2;

use self::blake2::{Blake2b, Digest};
use self::blake2::digest::generic_array::GenericArray;
use self::blake2::digest::generic_array::typenum::U64;

pub const EDGEBITS: i32 = 19;
pub const NEDGES: i32 = 1 << EDGEBITS;
pub const NODEBITS: i32 = EDGEBITS + 1;
pub const NNODES: i32 = 1 << NODEBITS;
pub const EDGEMASK: i32 = NEDGES - 1;
pub const PROOFSIZE: usize = 42;

#[derive(Debug, Eq)]
pub struct Edge {
    pub u: i32,
    pub v: i32,
}

impl Edge {
    pub fn hashcode(&self) -> i32 {
        return self.u ^ self.v;
    }
}

impl PartialEq for Edge {
    fn eq(&self, other: &Edge) -> bool {
        return self.u == other.u && self.v == other.v;
    }
}

fn u8(a: u8) -> u64 {
    return (a as u64) & 0xff;
}

fn u8to64(p: GenericArray<u8, U64>, i: usize) -> u64 {
    return u8(p[i]) | u8(p[i + 1]) << 8 | u8(p[i + 2]) << 16 | u8(p[i + 3]) << 24
        | u8(p[i + 4]) << 32 | u8(p[i + 5]) << 40 | u8(p[i + 6]) << 48
        | u8(p[i + 7]) << 56;
}

pub fn hash_header(header: &[u8]) -> [u64; 4] {
    let mut hasher = Blake2b::new();
    hasher.input(header);
    let result = hasher.result();
    return [
        u8to64(result, 0),
        u8to64(result, 8),
        u8to64(result, 16),
        u8to64(result, 24),
    ];
}

#[inline]
fn rotl(x: u64, b: u64) -> u64 {
    return ((x) << (b)) | ((x) >> (64 - (b)));
}

#[inline]
fn sipround(v0: &mut u64, v1: &mut u64, v2: &mut u64, v3: &mut u64) {
    *v0 += *v1;
    *v2 += *v3;
    *v1 = rotl(*v1, 13);

    *v3 = rotl(*v3, 16);
    *v1 ^= *v0;
    *v3 ^= *v2;

    *v0 = rotl(*v0, 32);
    *v2 += *v1;
    *v0 += *v3;

    *v1 = rotl(*v1, 17);
    *v3 = rotl(*v3, 21);

    *v1 ^= *v2;
    *v3 ^= *v0;
    *v2 = rotl(*v2, 32);
}

pub fn siphash24(v: [u64; 4], nonce: u64) -> u64 {
    let mut v0: u64 = v[0];
    let mut v1: u64 = v[1];
    let mut v2: u64 = v[2];
    let mut v3: u64 = v[3] ^ nonce;

    sipround(&mut v0, &mut v1, &mut v2, &mut v3);
    sipround(&mut v0, &mut v1, &mut v2, &mut v3);

    v0 ^= nonce;
    v2 ^= 0xff;

    sipround(&mut v0, &mut v1, &mut v2, &mut v3);
    sipround(&mut v0, &mut v1, &mut v2, &mut v3);
    sipround(&mut v0, &mut v1, &mut v2, &mut v3);
    sipround(&mut v0, &mut v1, &mut v2, &mut v3);
    return (v0 ^ v1) ^ (v2 ^ v3);
}

pub fn sipnode(v: [u64; 4], nonce: i32, uorv: i32) -> i32 {
    return (siphash24(v, (2 * nonce + uorv) as u64) as i32) & EDGEMASK;
}

pub fn sipedge(v: [u64; 4], nonce: i32) -> Edge {
    return Edge {
        u: sipnode(v, nonce, 0),
        v: sipnode(v, nonce, 1),
    };
}

pub fn verify(v: [u64; 4], nonces: [i32; PROOFSIZE], easiness: i32) -> bool {
    let mut us: [i32; PROOFSIZE] = [0; PROOFSIZE];
    let mut vs: [i32; PROOFSIZE] = [0; PROOFSIZE];

    let mut i: usize = 0;

    for n in 0..PROOFSIZE {
        if nonces[n] >= easiness || (n != 0 && nonces[n] <= nonces[n - 1]) {
            return false;
        }
        us[n] = sipnode(v, nonces[n], 0);
        vs[n] = sipnode(v, nonces[n], 1);
    }

    let mut n: usize = PROOFSIZE;

    loop {
        let mut j: usize = i;
        for k in 0..PROOFSIZE {
            // find unique other j with same vs[j]
            if k != i && vs[k] == vs[i] {
                if j != i {
                    return false;
                }
                j = k;
            }
        }
        if j == i {
            return false;
        }
        i = j;
        for k in 0..PROOFSIZE {
            // find unique other i with same us[i]
            if k != j && us[k] == us[j] {
                if i != j {
                    return false;
                }
                i = k;
            }
        }
        if i == j {
            return false;
        }
        n -= 2;

        if i != 0 {
            break;
        }
    }
    return n == 0;
}
