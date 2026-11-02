//! Rigorous ISA §17 item 8 sin/cos enclosures. Arguments are exact binary64
//! rationals, reduced modulo 2π using a proved outward π enclosure. Taylor
//! arithmetic rounds outward at the requested precision; the first omitted
//! term supplies the Lagrange remainder (including the intervening zero term).
use crate::fault::HostLimit;
use crate::rat::{round_down,round_up,scale_pow2,Interval,Rat};
use num_bigint::BigInt;
use num_traits::{One,Signed,Zero};

pub fn binary64_rational(bits:u64)->Result<Rat,String>{
    let exponent=(bits>>52)&0x7ff;let fraction=bits&((1u64<<52)-1);
    if exponent==0x7ff{return Err("gate angle is NaN or infinite (§3.1)".into());}
    let mantissa=if exponent==0{fraction}else{fraction|(1u64<<52)};
    let shift=if exponent==0{-1074}else{exponent as i64-1023-52};
    let mut value=scale_pow2(&Rat::from_integer(mantissa.into()),shift);
    if bits>>63!=0{value=-value;}Ok(value)
}
fn host(quantity:impl Into<String>)->HostLimit{HostLimit{quantity:quantity.into()}}
fn precision(p:u32)->Result<(),HostLimit>{if p==0 || p>4096{Err(host(format!("transcendental precision {p} bits (host range 1..4096)")))}else{Ok(())}}
fn atan_reciprocal(n:u32,tolerance:&Rat)->Interval{
    // Alternating arctan series on x=1/n, 0<x<1. Exact rational partial
    // sums and the next-term bound prove the constant before it is rounded.
    let x=Rat::new(1.into(),n.into());let x2=&x*&x;let mut power=x.clone();let mut sum=x;
    for k in 1u32..10000 {
        power*=&x2;let term=&power/Rat::from_integer((2*k+1).into());
        if &term<tolerance {
            return if k%2==1{Interval{lo:&sum-&term,hi:sum}}else{Interval{lo:sum.clone(),hi:sum+term}};
        }
        if k%2==1{sum-=term;}else{sum+=term;}
    }
    unreachable!("4096-bit arctan(1/5) and arctan(1/239) need fewer than 10000 terms")
}
pub fn pi(p:u32)->Result<Interval,HostLimit>{
    precision(p)?;
    // Machin identity: π=16 atan(1/5)-4 atan(1/239). Each alternating
    // remainder is rigorously bounded; no stored floating point π is used.
    let tolerance=scale_pow2(&Rat::one(),-i64::from(p)-8);
    let a=atan_reciprocal(5,&tolerance);let b=atan_reciprocal(239,&tolerance);
    let lo=&a.lo*Rat::from_integer(16.into())-&b.hi*Rat::from_integer(4.into());
    let hi=&a.hi*Rat::from_integer(16.into())-&b.lo*Rat::from_integer(4.into());
    Ok(Interval{lo:round_down(&lo,p),hi:round_up(&hi,p)})
}
fn square(x:&Interval,p:u32)->Interval{
    if x.contains_zero(){Interval{lo:Rat::zero(),hi:round_up(&(x.max_abs()*x.max_abs()),p)}}
    else{x.mul(x,p)}
}
fn series(x:&Interval,sine:bool,p:u32)->Result<Interval,HostLimit>{
    let x2=square(x,p);let radius=Interval::point(x.max_abs(),p);let r2=radius.mul(&radius,p);
    let mut term=if sine{x.clone()}else{Interval::exact(Rat::one())};let mut sum=term.clone();
    let mut magnitude=if sine{radius}else{Interval::exact(Rat::one())};let mut order=if sine{1u64}else{0};
    let tolerance=scale_pow2(&Rat::one(),-i64::from(p)-2);
    for _ in 0..4096 {
        let denominator=Interval::exact(Rat::from_integer(((order+1)*(order+2)).into()));
        let next=magnitude.mul(&r2,p).div(&denominator,p).expect("positive factorial factor");
        if next.hi<tolerance {
            let remainder=Interval{lo:-next.hi.clone(),hi:next.hi};let mut answer=sum.add(&remainder,p);
            // Exact range information can only tighten an enclosure.
            answer.lo=answer.lo.max(-Rat::one());answer.hi=answer.hi.min(Rat::one());return Ok(answer);
        }
        term=term.mul(&x2,p).div(&denominator,p).expect("positive factorial factor").neg();
        sum=sum.add(&term,p);magnitude=next;order+=2;
    }
    Err(host("sin/cos Taylor series exceeds 4096 host terms before its rigorous Lagrange remainder is small enough"))
}
pub fn sin_cos(angle:&Rat,p:u32)->Result<(Interval,Interval),HostLimit>{
    precision(p)?;
    if angle.is_negative(){let (s,c)=sin_cos(&-angle,p)?;return Ok((s.neg(),c));}
    if angle.is_zero(){return Ok((Interval::exact(Rat::zero()),Interval::exact(Rat::one())));}
    let reduced=if angle<=&Rat::from_integer(3.into()){Interval::point(angle.clone(),p)}else{
        let pi=pi(p)?;
        let midpoint_period=&pi.lo+&pi.hi;
        let k=(angle/midpoint_period+Rat::new(1.into(),2.into())).floor().to_integer();
        let period=pi.mul(&Interval::exact(Rat::from_integer(2.into())),p);
        Interval::point(angle.clone(),p).sub(&Interval::point(Rat::from(k),p).mul(&period,p),p)
    };
    if reduced.max_abs()>Rat::from_integer(BigInt::from(8)) {
        return Err(host(format!("rotation argument reduction at {p} bits leaves an interval outside [-8,8]; π and cancellation uncertainty exceed this host's Taylor work bound")));
    }
    Ok((series(&reduced,true,p)?,series(&reduced,false,p)?))
}
