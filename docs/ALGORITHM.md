# Algorithms

This document serves to explain about the algorithms used for search and potential pitfalls. 

## Basics of the search

The base algorithm of the search used is the linear search, the one that iterates through every item and then compares a the value to the target (it is that simple). Many might question themselves: why use this algorithm and not something like [boyer-moore](https://en.wikipedia.org/wiki/Boyer%E2%80%93Moore_string-search_algorithm) (which is used by things like grep)? Well, because it is very simple to implement and I already had to deal with many other complexities of parallelizing the search.

Now that you know the basic algorithm, the thing that it is actually implemented is a parallel linear search: the search space is broken down into smaller pieces that will be sent to each thread to performa a linear search. This allows me to search huge amounts of memory very fast an efficiently.

In a more practical example: you have an array with 100 positions/values and you want the all of positions of the target value 10. In the sequential linear seach, one thread iterates over each position, compare it with the target and store the match position if it exists. In the parallel implementation, you divide the search space between each thread and perform the search. So let's assume 4 threads, each will iterate over 25 positions<sup>1</sup> to find the value and possibly reduce the search time by a factor of 4 <sup>2</sup>.

---
**FOOTNOTES**

1. I am assuming here that each thread will execute exclusively in equal CPU cores (so the cores are all equally powerfull). Modern architectures such as AMD's 3D cache or Intel's hybrid design (P cores and E cores) might benefit from another partitioning method.

1. I am assuming the task was perfectly parellel, it almost never is. This is just the upper bound limit.
---

## Endianess

> [!NOTE]
> I will start by saying that you don't need to worry about this, except if you want to cross-compile this program to multiple architectures (it will 100% bite your ass if you ignore endianess in this case).

So, let's starating by defining what endianess is: it is the ordering used to organize bytes of a variable in memory. Besides, there are 2 general models: big endian and little endian. Big Endian stores the value in a format closer to what we are used to reading (left to right): the Most Significant Byte (MSB) in the lowest address and the Least Significant Byte (LSB) in the highest address. Little Endian is the opposite of Big Endian: MSB at the lowest address and LSB at the higest.

Here is a representation of the number 134,480,385 as an unsigned 32bit integer in binary (this number is used because each byte is unique and helps to visualize - each 1 is in a different position):

**Normal writing**

|              |           |           |           |           |
|--------------|-----------|-----------|-----------|-----------|
| Power        | 2<sup>31</sup> - 2<sup>24</sup> | 2<sup>23</sup> - 2<sup>16</sup> | 2<sup>15</sup> - 2<sup>8</sup> | 2<sup>7</sup> - 2<sup>0</sup> |
| Value in bits|            0000 1000            |            0000 0100            |            0000 0010           |            0000 0001            |
||

<br>

**Big Endian**
|              |           |           |           |           |
|--------------|-----------|-----------|-----------|-----------|
| Address      |     0     |     1     |     2     |     3     |
| Value in bits| 0000 1000 | 0000 0100 | 0000 0010 | 0000 0001 |
||

<br>

**Little Endian**
|              |           |           |           |           |
|--------------|-----------|-----------|-----------|-----------|
| Address      |     0     |     1     |     2     |     3     |
| Value in bits| 0000 0001 | 0000 0010 | 0000 0100 | 0000 1000 |
||


I would like to point out to the reader an important detail that might be missed: the endianess deals with the order of BYTES and not BITs. So don't try to reorder the bits when trying to convert or understand endianess.

Therefore, it is particaularly important for a memory scanner to understand the endianess being used, otherwise it will start to interpret the values wrongly and return bogus results to the user. The endianess in this program is corrently handled automatically by the compiler, it does not have any runtime flags. So, you will need to compile the program to specific architectures for it to work correctly.

## Size of the value in bytes and overflow segments in parallel search

If you understood the basics, you may have noticed a problem with segmenting the search and type sizes: the naive approach misses some possible matches (compared to the sequential). So let's first take a look at an example with a byte search:

* It searches byte ber byte
* The buffer has 8 positions
* It uses 2 thread
* It looks for the 0x01 pattern (or 0000 0001)

1. It will divide the search space in 2
```
 Thread 0                                  Thread 1
starts here                               starts here
    |                                   |     |
    v                                   |     v
+ ----- + + ----- + + ----- + + ----- + | + ----- + + ----- + + ----- + + ----- +
| 0x00  | | 0x00  | | 0x00  | | 0x01  | | | 0x00  | | 0x00  | | 0x00  | | 0x00  |
+ ----- + + ----- + + ----- + + ----- + | + ----- + + ----- + + ----- + + ----- +
                                        |
                                        |
```

2. Each thread will now iterate each byte in their respective private space and check if a match was found
```
 Thread 0                        ends      Thread 1                       ends
starts here                      here     starts here                     here
    |                             |     |     |                             |
    v                             v     |     v                             v
+ ----- + + ----- + + ----- + + ----- + | + ----- + + ----- + + ----- + + ----- +
| 0x00  | | 0x00  | | 0x00  | | 0x01  | | | 0x00  | | 0x00  | | 0x00  | | 0x00  |
+ ----- + + ----- + + ----- + + ----- + | + ----- + + ----- + + ----- + + ----- +
                                        |
                                        |
```

So far, all good. Thread 0 will find a match at the 4fh position and return the result to us. However we also want to search for typer bigger than 1 byte, and this is where the naive approach starts to have problems. So let's repeat the experiment but look for 0x0101 (000 0001  0000 0001) pattern:

1. It will divide the search space in 2 like before. Note that the pattern was also divided in half.
```
 Thread 0                                  Thread 1
starts here                               starts here
    |         |                         |     |         |
    v         v                         |     v         v
+ ----- + + ----- + + ----- + + ----- + | + ----- + + ----- + + ----- + + ----- +
| 0x00  | | 0x00  | | 0x00  | | 0x01  | | | 0x01  | | 0x00  | | 0x00  | | 0x00  |
+ ----- + + ----- + + ----- + + ----- + | + ----- + + ----- + + ----- + + ----- +
                                        |
                                        |
```

2. Just the next iteration of each thread
```
              |         |               |               |         |
              v         v               |               v         v
+ ----- + + ----- + + ----- + + ----- + | + ----- + + ----- + + ----- + + ----- +
| 0x00  | | 0x00  | | 0x00  | | 0x01  | | | 0x01  | | 0x00  | | 0x00  | | 0x00  |
+ ----- + + ----- + + ----- + + ----- + | + ----- + + ----- + + ----- + + ----- +
                                        |
                                        |
```

3. The final iteration of each thread if we want them to only access their private region
```
                        |         |     |                         |         |
                        v         v     |                         v         v
+ ----- + + ----- + + ----- + + ----- + | + ----- + + ----- + + ----- + + ----- +
| 0x00  | | 0x00  | | 0x00  | | 0x01  | | | 0x01  | | 0x00  | | 0x00  | | 0x00  |
+ ----- + + ----- + + ----- + + ----- + | + ----- + + ----- + + ----- + + ----- +
                                        |
                                        |
```

Now, none of the threads were able to find the pattern in memory, despite existing in the buffer. The closest thread 0 was able to get was 0x0001 (!=0x0101) and thread 1 was 0x0100 (!= 0x0101). But, if we repeat it once again with a sequential algorithm, we will find it:

3. The sequential search. It succesfully finds the pattern in the buffer! (you can ignore the the other possible iterations)
* Observation: it still advances byte per byte, so it doesn't miss any possibility
```
                                Forth iteration
                                 (match found)
                                  |         |
                                  |         |
                      Third iteration       |
                        |         |         |
                        |         |         |
            Second iteration      |         |
              |         |         |         |
              |         |         |         |
   First iteration      |         |         |
    |         |         |         |         |
    v         v         v         v         v
+ ----- + + ----- + + ----- + + ----- + + ----- + + ----- + + ----- + + ----- +
| 0x00  | | 0x00  | | 0x00  | | 0x01  | | 0x01  | | 0x00  | | 0x00  | | 0x00  |
+ ----- + + ----- + + ----- + + ----- + + ----- + + ----- + + ----- + + ----- +

```

### The solution

Overflow the memory segments: read the necessary bytes of the other segment

Caution: the last segment cannot be overflowed, because that would be an actual buffer overflow
    * Do not add checks at every iteration
    * Only ajust the final segment size when you perform the partitioning

## Complexity

Worst case complexity: O(n)

* "n" being the mumber of bytes that needs to be searched

<br>

Actual number of necessary iterations per buffer: buffer_size_bytes - (type_size - 1)

* Type_size: the size in bytes of the type being searched, for example i32 (signed 32bit integer) is 4 bytes long.

