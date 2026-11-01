#![no_std]

#[unsafe(no_mangle)]
pub extern "C" fn r1_sat_add(a: i8, b: i8) -> i8 { a.saturating_add(b) }
#[unsafe(no_mangle)]
pub extern "C" fn r1_popcount(x: i8) -> i8 { x.count_ones() as i8 }
#[unsafe(no_mangle)]
pub extern "C" fn r1_gcd(a: i8, b: i8) -> i8 {
    let (mut x, mut y) = (a.unsigned_abs(), b.unsigned_abs());
    while y != 0 { let r = x % y; x = y; y = r; }
    x as i8
}
#[unsafe(no_mangle)]
pub extern "C" fn r1_sign_abs(x: i8) -> i8 {
    if x == i8::MIN { 127 } else if x < 0 { -x } else if x == 0 { -1 } else { x / 2 }
}
#[unsafe(no_mangle)]
pub extern "C" fn r1_lookup(x: i8) -> i8 {
    const TABLE: [i8; 8] = [3, -7, 11, 0, 127, -128, 42, -1];
    TABLE[(x as u8 & 7) as usize]
}
