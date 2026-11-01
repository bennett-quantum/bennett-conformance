function r1_sat_add(a::Int8, b::Int8)::Int8
    Int8(clamp(Int16(a) + Int16(b), -128, 127))
end
r1_popcount(x::Int8)::Int8 = Int8(count_ones(x))
function r1_gcd(a::Int8, b::Int8)::Int8
    x, y = abs(Int16(a)), abs(Int16(b))
    while y != 0
        x, y = y, rem(x, y)
    end
    x % Int8
end
function r1_sign_abs(x::Int8)::Int8
    x == -128 && return 127
    x < 0 && return -x
    x == 0 && return -1
    div(x, Int8(2))
end
function r1_lookup(x::Int8)::Int8
    table = (Int8(3), Int8(-7), Int8(11), Int8(0), Int8(127), Int8(-128), Int8(42), Int8(-1))
    table[Int(reinterpret(UInt8, x) & 7) + 1]
end
