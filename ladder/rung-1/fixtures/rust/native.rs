extern crate r1;
use std::io::{self, Write};
fn main() {
    let mut out = Vec::new();
    for k in 0..5 { for a in 0..256 { for b in 0..if k < 2 {256} else {1} {
        let (x,y) = (a as i8,b as i8);
        let v = match k { 0 => r1::r1_sat_add(x,y), 1 => r1::r1_gcd(x,y),
            2 => r1::r1_popcount(x), 3 => r1::r1_sign_abs(x), _ => r1::r1_lookup(x) };
        out.push(v as u8);
    }}}
    io::stdout().write_all(&out).unwrap();
}
