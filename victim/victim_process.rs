// How to compile
// rustc path_to_program

use std::io;
use std::process;

fn main()
{
    println!("PID: {}", process::id());

    let mut num: u32 = 0;
    let mut vector: Vec<u8> = vec![0; 1024];
    let vector_len: usize = vector.len();
    vector[vector_len-1] = 1;

    loop
    {
        num += 1;
        let mut guess = String::new();
        println!("Num: {} -- Address: {:p}", num, &num);

        io::stdin().read_line(&mut guess).expect("failed to readline");
    }
}