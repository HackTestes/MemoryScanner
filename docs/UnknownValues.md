# Unknown values search considerations

This document outlines some implementation considerations for the unknown value search. The goal here is to aid in future decisions, so when we look back, we can see why some paths were taken and others weren't.

## Memory optimizations

### Store an address per match or a range

One of the first questions when it come to search is: how are we going to store the matches? And since this type of search deals with huge memory segments, two types of storage estrategies can be utilized:

1. Every address matched gets stored;
2. We only store a contiguous segment of the matched addresses (which we will call the "range strategy").

The range strategy might seem as the best option without any drawbacks, but it isn't necessarily true. The memory benefits will depend heavily if the matches are too fragmented or not.

Before looking at some examples, we need to establish how the storage works for each data type:

* Every address stored must be a 64 bit (8 bytes) pointer, total of 8 bytes per match;
* A range stores the starting point and the size with a 64 bit interger for each and another one to the structure holding the range. However, the compiler might optimize this pointer, so we get a total of 16 or 24 bytes per match.

Now let's look at some examples.

1. Example 1
    * 1 Gib of data
    * All adresses matches
    * Type is u8 (unsigned integer of 1 byte)

* Memory usage
    * Address per match: 8 GiB
    * Range: 24 bytes or 16 bytes

Here the **range** has the clear benefit.

---

2. Example 2
    * 1 Gib of data
    * 1 match every 2 bytes (M#) - half of the memory matches
    * Type is u8 (unsigned integer of 1 byte)

* Memory usage
    * Address per match: 4 GiB (512 MiB x 8)
    * Range: 12 GiB or 8 GiB (512 x 24 or 512 x 16)

Here the **address per match** uses the least amount of memory. The range can use up to 3 times as much memory.

You can notice that this pattern wastes up to 2 extra bytes per match with the range strategy.

```
M -> address (1 byte) | range start and end (2 bytes)
# -> 0 bytes
```

---

3. Example 3
    * 1 Gib of data
    * 2 matches every 4 bytes (MM##)
    * Type is u8 (unsigned integer of 1 byte)

* Memory usage
    * Address per match: 4 GiB (512 MiB x 8)
    * Range: 6 GiB or 4 GiB (256 x 24 or 256 x 16)

> [!NOTE]
> The range uses *256* because the results are contiguous and this reduces the number of starting addresses. You can also check the numbers by looking how much memory is "wasted" in the pattern at the end.

Here the range might be the **same** as the address per match depending on the implementation.

In this pattern the range might only waste a single byte per match (because of a possible structure pointer, otherwise they use the same amout of memory).

```
M -> address (1 byte) | range start (1 byte)
M -> address (1 byte) | range end (1 byte)
# -> 0 bytes
# -> 0 bytes
```

---

4. Example 4
    * 4 Gib of data
    * 3 matches every 4 bytes (MMM#)
    * Type is u8 (unsigned integer of 1 byte)

* Memory usage
    * Address per match: 24 GiB (3GiB x 8)
    * Range: 24 GiB or 16 GiB (1 GiB x 24 or 1 GiB x 16)

In this pattern, the range is the **same** or better than an address per match. If we factor in the pointer (worst case), the range uses the same amount of memory as an address per match.

```
M -> address (1 byte) | range start (1 byte)
M -> address (1 byte) | 0 bytes
M -> address (1 byte) | range end (1 byte)
# -> 0 bytes
```

---

From the examples, we can conclude that only the "M#" is the most problematic for the range strategy and that we can get no additional overhead (compared to an address per match) at "MM##".

We can also infer that the initial results will return large contiguous segments and that subsequent scans will retrun less results, but more fragmented. This means that initial scans favor ranges and the subsequent scans will create more overhead, but it will be mitigated by having less results.

Therefore, using the range strategy really seems like the best approach for minimizing memory usage.

### Data types

Using a data type can help reduce the number of matches when searching for changed or unchanged values.

## Compute optimizations

### Multithreading (WIP)

Too much paralellization can cause serialization at the merging.


The number of iterations a single CPU core needs to perform in order to search its private region and merge all results. The optimal number of threads is the minimal of this rational function.
$$
interations\ per\ threads = \frac{numItems}{numThreads} + numThreads
$$

#### Partitioning the data

Same algorithm as the known value search Each thread reads a private section of a giver page like this:

Thread 1 - 0:100
Thread 2 - 100:200
...

#### Merging

The main thread must read the first and final results of each thread and verify if they are contiguous. If they are, they should be merged into a single result.

## Data size for unchanged values

It is possible to convert an u8 search for an unknown unchaged value to a bigger type, because all bytes will necessarily be the same. Changed values however must always use the same type for a scan.

## Converting ranges matches into direct addresses (unknown to known search)