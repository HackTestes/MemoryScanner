# Optimizations

This document is a quick summary of some specific optimizatioons used

1. Thread pool
1. Searching without syncing between memory regions (you could read the entire memory in one fell swoop if you have enough RAM)
1. Multithreaded copy
1. 