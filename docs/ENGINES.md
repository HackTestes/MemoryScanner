# Engines

This document serves to explain how some of the main engines works in a high level, providing users with a better understanding of the inner works of the program. 

## Exact number/match engine

The exact match engine will accept a binary array or a number of any type transformed into its array form and search for an exact match inside of the target program's writable memory. It is mostly powered by running the Rust's binary regex engine under multiple threads. This engine is well suited to look for strings, integers and exact structures (floats being a weakness since many only show approximate values).


## Comparison number engine

The comparison number engine take as arguments a comparison operation and a number, for example: equal to 10 or higher than 10. The main ideia is to copy fractions of the target program's writable memory and then transform it into a number type for the comparisons. This engine is particularly useful for searching float numbers as their exact values might not be displayed for the end user, however the user can still look for a range.

Let's take for example a program that shows a float as 99 to the end user, but it is in reality 99.1257. Searching for an exact match of 99.0 would weild no results, but searching for a number equal or higher than 99 and lower than 100 would sucessfully find it!


## Unknown number engine