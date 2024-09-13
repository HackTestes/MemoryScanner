// How to compile
// rustc path_to_program

use std::io;
use std::process;

fn number_loop()
{
    let mut num: u32 = 0;
    loop
    {
        num += 1;
        let mut guess = String::new();
        println!("Num: {} -- Address: {:p}", num, &num);

        io::stdin().read_line(&mut guess).expect("failed to readline");
    }
}

fn main()
{
    println!("PID: {}", process::id());


    // Contiguous section
    // rustc --cfg 'mem=\"cm\"' (Windows)
    #[cfg(mem="cm")]
    {
        println!("Contiguous memory");
        let mut vector: Vec<u8> = vec![1; 8*1024*1024*1024];
        let vector_len: usize = vector.len();
        vector[vector_len-1] = 2;

        number_loop();
    }


    // Non contiguous section
    // rustc --cfg 'mem=\"ncm\"' (Windows)
    #[cfg(mem="ncm")]
    {
        println!("Non-contiguous memory");
        let mut vector: Vec<Vec<u8>> = vec![];
        for _ in 0..16
        {
            vector.push( vec![1; 512*1024*1024] );
        }

        number_loop();
    }
}