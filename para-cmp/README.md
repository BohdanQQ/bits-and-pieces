# `para-cmp`

Quick binary comparison. C++20.

`para-cmp f1 f2 [threadCount] [totalMemoryBytes]`

## quick and dirty comparisons

Windows (5800H, Kingston KC3000 2TB SSD), 32GB files:

`Measure-Command { echo "N" | comp .\32G .\32G.2 }` yields: 123s

(2 threads, 8129B of buffer (total, 4096 per thread)):

`Measure-Command { .\para-cmp.exe .\32G .\32G.2 2 8192 }` yields: 27s
