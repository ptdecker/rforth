\ Stage-zero batch self-test for the minimal source-driven interpreter.

\ Equality should leave a true flag for equal operands.
1 1 = ?ABORT

\ Addition should produce the expected sum.
1 2 + 3 = ?ABORT ( Parenthesized comments should also be ignored. )

\ Subtraction should produce the expected difference.
5 2 - 3 = ?ABORT

\ DUP should copy the top data-stack value.
7 DUP = ?ABORT

\ DROP should discard only the top data-stack value.
5 6 DROP 5 = ?ABORT

\ SWAP should exchange the top two data-stack values.
1 2 SWAP - 1 = ?ABORT

\ OVER should copy the second data-stack value to the top.
1 2 OVER - 1 = ?ABORT

\ Cell store should write a full cell to VM memory.
99 4096 !

\ Cell fetch should read back the stored VM memory cell.
4096 @ 99 = ?ABORT

\ Byte store should write one byte to VM memory.
122 4097 C!

\ Byte fetch should read back the stored VM memory byte.
4097 C@ 122 = ?ABORT

\ Colon definitions should compile source tokens into dictionary words.
: THREE ( ignored while compiling ) 3 ;

\ The compiled colon definition should execute through the inner interpreter.
THREE 3 = ?ABORT

\ Dot should print the top data-stack value.
66 .

\ EMIT should write one byte as an output character.
88 EMIT

\ KEY should read the next raw input byte; the following X is input data for this test.
KEY 88 = ?ABORT
X
