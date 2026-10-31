Hand-derived expectations, recorded before execution:
while3: flag_i is AND of p[0] through p[i]. X flips t once per true flag.
Thus t is XORed with the parity of the initial consecutive ones in p, and p
is unchanged. At p=3 (011) it flips twice; at p=5 (101) it flips once.
Adjoint reverses those same enabled iterations and clears all three flags.
conditional: starting with t=0 and p=+ gives amplitudes [0,1/sqrt(2),1/2,1/2]
in allocation order (t low bit). Applying the adjoint restores t=0,p=+.
minus_i: S^2=Z; the six gates S S X S S X act as X Z X Z = -I exactly.
controlled_phase calls minus_i: controlled by p, it is Z on p;
H p; controlled_phase; H p therefore produces p=1 with probability one.
phase retains theta=6.283185307179586=884279719003555/140737488355328,
a finite binary64 angle, not symbolic 2*pi. Its Rz is
diag(exp(-i*theta/2), exp(i*theta/2)). Starting at t=p=0, the separate
H p; ctl p phase; H p probe has amplitudes (1+exp(-i*theta/2))/2 on 00
and (1-exp(-i*theta/2))/2 on 01 (wire order t,p).
P(p=0)=cos(theta/4)^2 is strictly positive, approximately 3.749399456654644e-33;
both exact-support keys must survive even when a displayed component rounds.
exchange: qexchm swaps t with m[a] and leaves the other cell unchanged.
image: data symbol bytes (hex dead) has cell0=0xde and cell1=0xad. With t initially0x35 the
result is0xeb or0x98 respectively. Scratch and local image are restored.
borrowed: CX x,b; CX b,t; CX x,b; CX b,t gives t XOR x, restores b,
and has no dependence on b, including when b is entangled externally.
