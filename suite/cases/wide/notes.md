Hand expectations, recorded before execution:
x=2^200+3=1606938044258990275541962092341162602522202993782792835301379;
y=2^100+5=1267650600228229401496703205381.
div257 quotient=2^100-5=1267650600228229401496703205371, remainder28,
since (2^100-5)(2^100+5)=2^200-25. It finishes in2 rows; B1 faults1.
add257 computes (x+y) mod2^257 in2 rows.
call257 captures x into id257, returns x, then retval:3 rows. id257's frame
has3 metadata+1 parameter+1 result=5 cells. stack4 faults3 on the first row;
stack5 and depth1 suffice. Every complete prefix must reverse.
fault257 emits code2^200+3 in one row, no payload, high bit200 retained.
fault_small257 has the same wide fault but one input bit: both inputs yield
that fault in one row. Its small input domain permits exhaustive CLI checking.
