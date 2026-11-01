include(joinpath(@__DIR__, "scalars.jl"))
using InteractiveUtils
mkpath(ARGS[1])
for (k, f) in enumerate((r1_sat_add, r1_gcd, r1_popcount, r1_sign_abs, r1_lookup))
    open(joinpath(ARGS[1], string(f) * ".ll"), "w") do io
        code_llvm(io, f, k <= 2 ? Tuple{Int8,Int8} : Tuple{Int8};
                  raw=true, dump_module=true, optimize=true, debuginfo=:none)
    end
end
println("Julia ", VERSION, "; LLVM ", Base.libllvm_version)
