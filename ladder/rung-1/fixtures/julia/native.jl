include(joinpath(@__DIR__, "scalars.jl"))
out = UInt8[]
for (k, f) in enumerate((r1_sat_add, r1_gcd, r1_popcount, r1_sign_abs, r1_lookup))
    for a in 0:255, b in 0:(k <= 2 ? 255 : 0)
        x, y = reinterpret(Int8, UInt8(a)), reinterpret(Int8, UInt8(b))
        push!(out, reinterpret(UInt8, k <= 2 ? f(x,y) : f(x)))
    end
end
write(stdout, out)
