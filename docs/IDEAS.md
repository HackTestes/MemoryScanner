# IDEAS

Just want to document some ideas to (maybe) use in the future

## Interlaced search

Insted of dividing the search space between all threads as equal slices, one cloud simply read a chunk of memory in one go.

* GOAL: Improve the usage of the data cache in the CPU
* WHY: This might improve cache usage by using values that are close to each other

Visualization

* Usual way
    * 2 threads
    * Divide the search space in two private segments
```
 Thread 0                                  Thread 1
  starts                                    starts
   here                                      here
    |                                   |     |                              
    v                                   |     v                              
+ ----- + + ----- + + ----- + + ----- + | + ----- + + ----- + + ----- + + ----- +
| 0x00  | | 0x00  | | 0x00  | | 0x01  | | | 0x01  | | 0x00  | | 0x00  | | 0x00  |
+ ----- + + ----- + + ----- + + ----- + | + ----- + + ----- + + ----- + + ----- +
                                        |
                                        |
```

* The new proposal
    * 2 threads
    * The is no private segment anymore
    * Each iteration steps based on the number of threads
```
           Thread 1
            starts
             here
              |
 Thread 0     |
  starts      |
   here       |
    |         |                                                             
    v         v                                                             
+ ----- + + ----- + + ----- + + ----- + + ----- + + ----- + + ----- + + ----- +
| 0x00  | | 0x00  | | 0x00  | | 0x01  | | 0x01  | | 0x00  | | 0x00  | | 0x00  |
+ ----- + + ----- + + ----- + + ----- + + ----- + + ----- + + ----- + + ----- +
                                        
                                        
```

* The next iteration
    * It simply steps based on the number of threads (2)
    * start_pos + num_threads
    * T0: 0 + 2 = 2
    * T1: 1 + 2 = 3
```
                               Thread 1
                                  |
                                  |
                                  |
                     Thread 0     |
                        |         |                                                             
                        v         v                                                             
+ ----- + + ----- + + ----- + + ----- + + ----- + + ----- + + ----- + + ----- +
| 0x00  | | 0x00  | | 0x00  | | 0x01  | | 0x01  | | 0x00  | | 0x00  | | 0x00  |
+ ----- + + ----- + + ----- + + ----- + + ----- + + ----- + + ----- + + ----- +
                                        
                                        
```

* The number of iterations can be calculated based on the buffer size and the number of threads
    * n_iter = buffer_size / num_threads
    * n_iter = 8 / 2 = 4
    * Watch out for the size of the target and calculations with more threads


### Result merge

In order to merge results in order, you CANNOT simply merge the match arrays, you must visit the first elements of each thread result. Example:
```
merged_results = [];

LOOP UNTIL threads_results IS EMPTY
{
    FOR EACH thread_matches_queue IN threads_results
    {
        // thread_matches_queue[0]
        // The particular queue might be empty, returning NONE
        // The function removes the item from the queue
        first_elem = GET_FIRST_ELEMENT(thread_matches_queue);

        IF first_elem != NONE
        {
            merged_results.push(first_elem);
        }
    }
}
```