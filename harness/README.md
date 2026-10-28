# harness

Differential harness: runs each program natively and through the compiled
pipeline on the same inputs, compares results, and uses instrumented native
builds (UBSan, ASan, MSan) as classifiers that flag inputs outside the defined
domain D (North Star 5.1 part 1, 3.2). Flagged inputs are reported, never
silently dropped. Skeleton.
