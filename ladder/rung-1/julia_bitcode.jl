# Serialize code_llvm's unchanged text using the emitting Julia's own LLVM.
# The bitcode identification record then carries the actual producer version.
using Libdl
const llvm = only(filter(p -> occursin(r"libLLVM\.", basename(p)), dllist()))
for path in sort(filter(p -> endswith(p, ".ll"), readdir(ARGS[1]; join=true)))
    context = ccall((:LLVMContextCreate, llvm), Ptr{Cvoid}, ())
    buffer = Ref{Ptr{Cvoid}}(C_NULL)
    message = Ref{Ptr{UInt8}}(C_NULL)
    module_ref = Ref{Ptr{Cvoid}}(C_NULL)
    try
        rc = ccall((:LLVMCreateMemoryBufferWithContentsOfFile, llvm), Cint,
                   (Cstring, Ref{Ptr{Cvoid}}, Ref{Ptr{UInt8}}), path, buffer, message)
        rc == 0 || error("LLVM buffer: ", unsafe_string(message[]))
        rc = ccall((:LLVMParseIRInContext, llvm), Cint,
                   (Ptr{Cvoid}, Ptr{Cvoid}, Ref{Ptr{Cvoid}}, Ref{Ptr{UInt8}}),
                   context, buffer[], module_ref, message)
        rc == 0 || error("LLVM parse: ", unsafe_string(message[]))
        rc = ccall((:LLVMWriteBitcodeToFile, llvm), Cint,
                   (Ptr{Cvoid}, Cstring), module_ref[], path[1:end-3] * ".bc")
        rc == 0 || error("LLVM bitcode write failed: ", path)
    finally
        message[] == C_NULL || ccall((:LLVMDisposeMessage, llvm), Cvoid, (Ptr{UInt8},), message[])
        module_ref[] == C_NULL || ccall((:LLVMDisposeModule, llvm), Cvoid, (Ptr{Cvoid},), module_ref[])
        ccall((:LLVMContextDispose, llvm), Cvoid, (Ptr{Cvoid},), context)
    end
end
