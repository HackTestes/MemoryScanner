// This should be a crate in the future

/*
# Considerations

    ## Moving closure API

    It is bound to be slower, because moving closures requires Fn() or FnOnce() and they require dyn (dinamic dispatch). It might also require Box to be able to store the pointer it into a struct. I have also made tests to verify if it was slower and I was able to confirm it (slightly slower).

    Another point, the clusure required some amount of unsafe code.
    
    Since I don't belive this style of API brings any benefits, moving closures won't be supported. It could be that my implementation was bad, but this was the best I could do.

    ## Variadic API

    Performance was identical to the one used here, but it required some usafe code and it was'nt ergonomic (it would need the use os macros). Since I like the current API and variadic did't bring any benefits, it won't be supportted.
*/

// Implementation of a thread pool to reduce the need of recreating threads all of the time
// It reduces the cost of thread creation syscall

use std::sync::mpsc;
use std::thread;
use std::mem;

#[derive(Debug)]
#[derive(PartialEq)]
pub enum TPErrors
{
    InvalidNumberOfThreads,
    ThreadAlreadyHasATask
}

// ThreadPool
// Actions
// Create a pool with a certain number of threads
// Execute individual tasks on each thread in the pool
// Wait for all threads in the pool to finish
// Destroy thread pool

// ARGS -> structure containing the data that will be used by the function, so it can have the same signature across the threads (function(args) - I can't add parameters dynamically)
// Also, it allows me to convert a closure to a function as it can't capture any outside values 
// A possible alternative is using dynamic dispatch(dyn), but I don't want the performance impact
// RETURN_STRUCT -> structure containing the information returned by the task function
struct TaskTP<ARGS, RETURN_STRUCT>
{
    arguments_struct: Option<ARGS>,
    function_ptr: Option< fn(args: ARGS) -> RETURN_STRUCT >,
    exit: bool // Used when terminating the pool instance
}

impl<ARGS, RETURN_STRUCT> TaskTP<ARGS, RETURN_STRUCT>
{
    fn new(args: ARGS, function: fn(args: ARGS) -> RETURN_STRUCT) -> TaskTP<ARGS, RETURN_STRUCT>
    {
        return TaskTP
        {
            arguments_struct: Some(args),
            function_ptr: Some(function),
            exit: false
        };
    }

    // Creates an empty task to inform the worker thread that it can exit now
    fn exit() -> TaskTP<ARGS, RETURN_STRUCT>
    {
        return TaskTP
        {
            arguments_struct: None,
            function_ptr: None,
            exit: true
        };
    }

    // Borrow the function pointer - read-only
    fn get_func_ptr(&self) -> Result<&fn(args: ARGS) -> RETURN_STRUCT, String>
    {
        match &self.function_ptr
        {
            Some(func) => return Ok(func),
            None => Err("No function present".to_string()),
        }
    }

    // Borrow the args - read-only
    fn get_args(&self) -> Result<&ARGS, String>
    {
        match &self.arguments_struct
        {
            Some(args) => return Ok(args),
            None => Err("No arguments present".to_string()),
        }
    }

    // Takes ownsership of the arguments, consuming them (aka leaving None in the structure)
    // This is done so partial moving does not invalidate the whole object
    fn take_args(&mut self) -> Result<ARGS, String>
    {
        let arguments: Option<ARGS> =  mem::take(&mut self.arguments_struct);
        
        // Extract the args
        match arguments
        {
            Some(args) => return Ok(args),
            None => todo!()
        }
    }
}


// Thread from Thread Pool
// It represents a single thread in the pool
struct ThreadTP<ARGS, RETURN_STRUCT>
{
    handle: Option< thread::JoinHandle<Result<(), String>> >,
    assigned: bool, // Did main send a task?

    // Main sends tasks, worker receives them
    task_queue_sender: mpsc::Sender< TaskTP<ARGS, RETURN_STRUCT> >,
    task_queue_receiver: Option< mpsc::Receiver< TaskTP<ARGS, RETURN_STRUCT>> >,

    // Worker sends results, main receives them
    result_queue_sender: Option< mpsc::Sender<RETURN_STRUCT> >,
    result_queue_receiver: mpsc::Receiver<RETURN_STRUCT>
}

impl<ARGS, RETURN_STRUCT> ThreadTP<ARGS, RETURN_STRUCT>
{
    fn new() -> ThreadTP<ARGS, RETURN_STRUCT>
    {
        let (task_s, task_r) = mpsc::channel::< TaskTP<ARGS, RETURN_STRUCT> >();
        let (result_s, result_r) = mpsc::channel::<RETURN_STRUCT>();

        return ThreadTP
        {
            handle: None,
            assigned: false,
            task_queue_sender: task_s,
            task_queue_receiver: Some(task_r),
            result_queue_sender: Some(result_s),
            result_queue_receiver: result_r 
        };
    }

    // Borrow thread handle - read-only
    fn get_handle(&self) -> &thread::JoinHandle<Result<(), String>>
    {
        match &self.handle
        {
            Some(handle) => return handle,
            None => todo!(),
        }
    }

    // Takes ownsership of the receiver of the task queue, leaving None in the palce
    fn take_worker_task_receiver(&mut self) -> mpsc::Receiver< TaskTP<ARGS, RETURN_STRUCT>>
    {
        let receiver: Option< mpsc::Receiver< TaskTP<ARGS, RETURN_STRUCT>> > =  mem::take(&mut self.task_queue_receiver);
        
        match receiver
        {
            Some(recv) => return recv,
            None => todo!()
        }
    }

    // Takes ownsership of the sender of the result queue, leaving None
    // Main doesn't send any results, so the worker can have full ownsership 
    fn take_worker_result_sender(&mut self) -> mpsc::Sender<RETURN_STRUCT>
    {
        let sender: Option< mpsc::Sender<RETURN_STRUCT> > = mem::take(&mut self.result_queue_sender);
        
        match sender
        {
            Some(snd) => return snd,
            None => todo!()
        }
    }
}

fn print_type_of<T>(_: &T) {
    println!("{}", std::any::type_name::<T>());
}


// The actual pool, it controls all other substructures
pub struct ThreadPool<ARGS, RETURN_STRUCT>
{
    thread_list: Vec< ThreadTP<ARGS, RETURN_STRUCT> >,
}

// Send + 'static is required by thread::spawn -> they don't cause mem leaks as the underlying data gets deallocated
impl<ARGS: Send + 'static, RETURN_STRUCT: Send + 'static> ThreadPool<ARGS, RETURN_STRUCT>
{
    pub fn new(num_threads: usize) -> Result<ThreadPool<ARGS, RETURN_STRUCT>, TPErrors>
    {
        // Function fails with an invalid number of threads
        // In other words, you must create a pool with at least 1 thread
        // TODO: alert the user that values <=0 should make this function fail - would a Result be better?
        if num_threads < 1
        {
            return Err(TPErrors::InvalidNumberOfThreads);
        }

        // Thread list - holds the handles to each thread
        let mut new_thread_list: Vec< ThreadTP<ARGS, RETURN_STRUCT> > = Vec::new();

        for _idx in 0..num_threads
        {
            // Thread - contains general information about the thread
            let mut thread = ThreadTP::<ARGS, RETURN_STRUCT>::new();

            // Private reference for each thread
            let thread_task_rcv = thread.take_worker_task_receiver();
            let thread_result_sender = thread.take_worker_result_sender();

            let thread_handle = thread::spawn(move|| -> Result<(), String>
                {
                    loop
                    {
                        // Wait for main to send tasks
                        let mut task = thread_task_rcv.recv().unwrap();

                        // Is main asking to exit?
                        if task.exit == true
                        {
                            // If so, return
                            return Ok(());
                        }

                        // Execute the task sent
                        let results = ( task.get_func_ptr().unwrap() )( task.take_args().unwrap() );

                        // Send the return values back to main. It should also wake it up, if it is waiting
                        thread_result_sender.send(results).unwrap();
                    }
                });

            // Store the thread handle into the structure
            thread.handle = Some(thread_handle);

            // Store into the pool list
            new_thread_list.push(thread);
        }

        return Ok(ThreadPool
        {
            thread_list: new_thread_list,
        });
    }


    pub fn execute(&mut self, thread_id: usize, args: ARGS, task: fn(args: ARGS) -> RETURN_STRUCT) -> Result<(), TPErrors>
    {

        // One should not send tasks to already assinged threads
        if self.thread_list[thread_id].assigned == true
        {
            return Err(TPErrors::ThreadAlreadyHasATask);
        }

        // Sends a task to the thread
        self.thread_list[thread_id].task_queue_sender.send( TaskTP::new(args, task) );

        // Keep track that it has been started by main
        self.thread_list[thread_id].assigned = true;

        return Ok(());
    }

    pub fn wait_all(&mut self) -> Result<Vec<RETURN_STRUCT>, String>
    {
        let mut results: Vec<RETURN_STRUCT> = Vec::new();

        // Wait for the worker threads to finish
        for thread_id in 0..self.thread_list.len()
        {
            // Does it have work to do?
            if self.thread_list[thread_id].assigned == false
            {
                // No, it doesn't have any work. Then we don't have to wait for it
                continue;
            }

            // Has it finished? Wait for results
            results.push(self.thread_list[thread_id].result_queue_receiver.recv().unwrap());

            // Reset the environment, so it can accept new tasks
            self.thread_list[thread_id].assigned = false;
        }

        return Ok(results);
    }

    pub fn get_num_threads(&self) -> usize
    {
        return self.thread_list.len();
    }

    pub fn scope_execute<args_type: Send>(&mut self, mut input_args: Vec<args_type>, task: fn (args: ARGS) -> RETURN_STRUCT) -> Vec<RETURN_STRUCT>
    {
        for t_idx in 0..input_args.len()
        {
            println!("{}", std::any::type_name::<args_type>());
            print_type_of(&input_args);

            // A runtime check for the time. Therefore, it can build with the wrong type, but it will panic during execution
            assert_eq!(std::any::type_name::<args_type>(), std::any::type_name::<ARGS>());

            // Transmute the lifetime: or essentially lie about the object's lifetime, so if we don't wait, we will get memory errors
            // We need to transmute to be able to send the tasks with stable interfaces. Even std's scoped_threads use an undocumented API that spawns unchecked threads (at this moment)
            // ManuallyDrop is an early forget
            let arg = input_args.remove(0);
            let transmuted_args = unsafe{std::mem::transmute_copy::<args_type, ARGS>(&std::mem::ManuallyDrop::new(arg))};

            // Check for any errors: in this case panic, because the error might not be recoverable (and can't resend tasks if something did go wrong)
            self.execute(t_idx, transmuted_args, task).unwrap();
        }
    
        // Wait for all threads to finish and return the results
        self.wait_all().unwrap()
    }
}

impl<ARGS, RETURN_STRUCT> Drop for ThreadPool<ARGS, RETURN_STRUCT>
{
    fn drop(&mut self)
    {
        // Send the exit command to all threads
        // There is no need to wait for any of them
        // Calling join is possible, but would require the main to wait (reducing performance unnecessarily)
        // Therefore, all threads will be implicitly detached
        for thread_id in 0..self.thread_list.len()
        {
            // Wake up all idle threads
            // Threads that have some work will continue to do so. When they finish, they will see a new task and exit
            // Also, we can safely ignore the assigned field
            self.thread_list[thread_id].task_queue_sender.send( TaskTP::exit() );
        }
    }
}

macro_rules! scope_execute
{
    ($args_type: ty, $return_type: ty, $input_args: ident, $task: ident, $thread_pool: ident) =>
    {
        {
            // Type check the pool's types
            // Use concrete types to do late type checking, so I get warnings at compile time
            // Why use a macro? Because macros first expand to later check the types, while generics use the "generic type" as a type itself
            {let _: &ThreadPool::<$args_type, $return_type> = &$thread_pool;}

            for t_idx in 0..$input_args.len()
            {
                println!("{}", std::any::type_name::<$args_type>());
                print_type_of(&$input_args);
                // Transmute the lifetime: or essentially lie about the object's lifetime, so if we don't wait, we will get memory errors
                // We need to transmute to be able to send the tasks with stable interfaces. Even std's scoped_threads use an undocumented API that spawns unchecked threads (at this moment)
                let transmuted_args = unsafe{std::mem::transmute::<$args_type, $args_type>($input_args.remove(0))};

                // Check for any errors: in this case panic, because the error might not be recoverable (and can't resend tasks if something did go wrong)
                $thread_pool.execute(t_idx, transmuted_args, task).unwrap();
            }
        
            // Wait for all threads to finish and return the results
            $thread_pool.wait_all().unwrap()
        }
    }
}

// It is not possible to use barriers in this particular implementation, destroy might deadlock if only some threads were assigned work (pool of 10, only 5 have work)
// Picture this situation:
// some threads have some tasks --> blocked at barrier wait : to resume all other threads must rendevousz at barrier wait
// threads without tasks --> blocked at park : to resume we call unpark


// Unit tests
// They will also be used to show some ways to use the thread pool, so you can use it as a tutorial of sorts
#[cfg(test)]
mod tests
{
    // Import the current module to all tests
    use crate::ThreadPool::*;

    #[test]
    fn TestPool()
    {
        let num_threads: usize = 5;

        // Setup the thread pool with the function input, output and number of threads
        let mut thread_pool = ThreadPool::<(i32, i32), i32>::new(num_threads).unwrap();

        // You can create an actual function that will be executed by the threads
        // Note that the RETURN value must match the one at the creation of the pool
        fn task(arg: (i32, i32)) -> i32
        {
            // You can unpack the args struct inside of the function
            let (_arg1, arg2) = arg; 
            println!("Hello from task! Args: {:?}", arg);
            return arg2;
        }

        // Send the task to each thread
        for idx in 0..num_threads
        {
            // The pool will execute the function passed as as pointer at the selected thread
            // Note that the ARGUMENTS type must match the one used at the creation of the pool
            thread_pool.execute(idx, (idx as i32, (idx+1) as i32), task);
        }

        // All results are collected at once
        let all_results = thread_pool.wait_all().unwrap();
        println!("Results: {:?}", all_results);

        // The result should be an array of idx+1
        let expected_result: Vec<i32> = vec![1, 2, 3, 4, 5];
        assert_eq!(expected_result, all_results);
    }

    #[test]
    fn TestPoolClosure()
    {
        let num_threads: usize = 5;

        // Setup the thread pool with the function input, output and number of threads
        let mut thread_pool = ThreadPool::<(i32, i32), i32>::new(num_threads).unwrap();

        // Say you have a function that you can't change to conform for the new API, you can use a non-moving closure
        fn task(arg1: i32, arg2: i32) -> i32
        {
            println!("Hello from task! Args: {:?} - {:?}", arg1, arg2);
            return arg2;
        }

        // Send the task to each thread
        for idx in 0..num_threads
        {
            // The pool will execute the function passed as as pointer at the selected thread
            // Note that the ARGUMENTS type must match the one used at the creation of the pool
            // Now we will create a closure to adapt the interface and unpack the args before we cann the function
            // Note that Rust can infer the types
            thread_pool.execute( idx, (idx as i32, (idx+1) as i32), |args| {task(args.0, args.1)} );
        }

        // All results are collected at once
        let all_results = thread_pool.wait_all().unwrap();
        println!("Results: {:?}", all_results);

        // The result should be an array of idx+1
        let expected_result: Vec<i32> = vec![1, 2, 3, 4, 5];
        assert_eq!(expected_result, all_results);
    }

    // The goal is to verify if seding less tasks than threads will cause any deadlock
    #[test]
    fn TestPoolTasksLessThanThreads()
    {
        let num_threads: usize = 10;

        // Setup the thread pool with the function input, output and number of threads
        let mut thread_pool = ThreadPool::<i32, i32>::new(num_threads).unwrap();

        // Say you have a function that you can't change to conform for the new API, you can use a non-moving closure
        fn task(arg1: i32) -> i32
        {
            println!("Hello from task! Args: {:?}", arg1);
            return arg1;
        }

        // Send the task to each thread
        for idx in 0..num_threads
        {
            // Only some threads will get work
            if idx % 2 == 0
            {
                thread_pool.execute( idx, idx as i32, task );
            }
        }

        // All results are collected at once
        // It should skip the idle workers
        let all_results = thread_pool.wait_all().unwrap();
        println!("Results: {:?}", all_results);

        // The result should be an array of idx+1
        let expected_result: Vec<i32> = vec![0, 2, 4, 6, 8];
        assert_eq!(expected_result, all_results);
    }

    // The goal of this test was to verify if the arguments passed to the threads would leak any memory (because of the static lifetime requirement)
    // It did not showed any signs of leak
    #[allow(unreachable_code)]
    #[ignore]
    #[test]
    fn TestPoolArgsLeak()
    {
        //use::std::time;
        {
            let mut thread_pool = ThreadPool::<Vec<u8>, i32>::new(1).unwrap();

            fn task(_arg: Vec<u8>) -> i32
            { 
                return 0;
            }

            loop
            {
                let vector: Vec<u8> = vec![1; 1*1024*1024];

                let _ = thread_pool.execute(0 as usize, vector, task);
                let _all_results = thread_pool.wait_all();
            }

            //thread::sleep(time::Duration::from_millis(10000));
        }
        assert_eq!(false,true);
    }

    // The goal of this test was to verify if the return value passed by the threads would leak any memory (because of the static lifetime requirement)
    // It did not showed any signs of leak
    #[allow(unreachable_code)]
    #[ignore]
    #[test]
    fn TestPoolReturnLeak()
    {
        //use::std::time;
        {
            let mut thread_pool = ThreadPool::<i32, Vec<u8>>::new(1).unwrap();

            fn task(_arg: i32) -> Vec<u8>
            { 
                let vector: Vec<u8> = vec![1; 1*1024*1024];
                return vector;
            }

            loop
            {
                let _ = thread_pool.execute(0 as usize, 1, task);
                let _all_results = thread_pool.wait_all();
            }

            //thread::sleep(time::Duration::from_millis(10000));
        }
        assert_eq!(false,true);
    }

    // The goal of this test was to verify if the pool creation would leak any memory
    // It did not showed any signs of leak
    #[allow(unreachable_code)]
    #[ignore]
    #[test]
    fn TestPoolCreationLeak()
    {
        //use::std::time;
        {
            fn task(arg: i32) -> i32
            { 
                return arg;
            }

            loop
            {
                let mut thread_pool = ThreadPool::<i32, i32>::new(1).unwrap();
                let _ = thread_pool.execute(0 as usize, 1, task);
                let _all_results = thread_pool.wait_all();
            }

            //thread::sleep(time::Duration::from_millis(10000));
        }
        assert_eq!(false,true);
    }

    // The goal of this test is to verify how much faster this approach is
    // This test might take a while to run, so it is better to ignore it
    #[ignore]
    #[test]
    fn TestPoolPerformanceMeasure()
    {
        use::std::time;
        {
            // Number of tasks
            // Thing of each task some individual work that needs to synchronized at the end
            let num_tasks: usize = 1000000;

            // Creates a fast task
            fn task(arg: (i32, i32, i32, i32)) -> i32
            {
                return arg.0;
            }

            // The thread pool is create before we measure time, so we ignore the pool creation cost (can we reuse threads efficiently?)
            let mut thread_pool = ThreadPool::<(i32, i32, i32, i32), i32>::new(1).unwrap();

            // Thread pool cost measurement
            #[allow(unused_mut)] // For the commented code below
            let mut now = time::Instant::now();
            for _ in 0..num_tasks
            {
                let _ = thread_pool.execute(0 as usize, (1, 2, 3, 4), task);
                let _all_results = thread_pool.wait_all();
                // Results aren't printed to avoid the cost of println in the measurements
            }
            let thread_pool_time_elapsed = now.elapsed().as_millis();
            println!("Thread pool time elapsed: {}ms", thread_pool_time_elapsed);
            println!("Thread pool, tasks p/ milisec: {}t/ms", num_tasks/thread_pool_time_elapsed as usize);

            // Test with the traditional threads API
            /*
            now = time::Instant::now();
            for _ in 0..num_tasks
            {
                let thread_arg = 1;
                let thread_handle = thread::spawn(move ||
                    {
                        return task(thread_arg);
                    });

                let all_results = thread_handle.join();
            }
            let thread_spawn_time_elapsed = now.elapsed().as_millis();
            println!("Thread spwan time elapsed: {}ms", thread_spawn_time_elapsed);
            println!("Thread spawn, tasks p/ milisec: {}t/ms", num_tasks/thread_spawn_time_elapsed as usize);
            */

            // Test with STD implementation of scoped threads
            /*
            now = time::Instant::now();
            for _ in 0..num_tasks
            {
                let thread_arg = 1;
                let all_results = thread::scope(|s|
                    {
                        s.spawn(move||
                            {
                                return task(thread_arg);
                            });
                    });

            }
            let thread_scope_time_elapsed = now.elapsed().as_millis();
            println!("Scoped Thread time elapsed: {}ms", thread_scope_time_elapsed);
            println!("Scoped Thread, tasks p/ milisec: {}t/ms", num_tasks/thread_scope_time_elapsed as usize);
            */
        }

        // It fails so we can see the output
        assert_eq!(false,true);
    }

    // Threads currently panic the main one, however this behavior might be underdesireable to many.
    // Why is it like that?
    //      - Workers that panic leave the pool in an undifined state: it would be necessary to recreate the thread (such API doesn't exist at the moment) and resend the task from main
    //      - I prefer to handle potential panics at the worker: it should be faster, as it avoids the round trip to main and easier to code
    // Any alternatives?
    //      - Not panic the main thread, let it recreate the whole pool and resend the task (sounds hard to use)
    #[allow(unreachable_code)]
    #[test]
    #[should_panic]
    fn TestPanickingThreads()
    {
        let num_tasks: usize = 1;

        fn task(_arg: i32) -> i32
        {
            // Panic inside the thread
            panic!();
            return _arg;
        }

        let mut thread_pool = ThreadPool::<i32, i32>::new(2).unwrap();

        for idx in 0..num_tasks
        {
            let _ = thread_pool.execute(idx as usize, 1, task);

            // Main will panic when it unwraps the result from the channel
            let all_results = thread_pool.wait_all();
            println!("Results: {:?}", all_results);
        }
    }

    #[test]
    fn ThreadPoolGetNumberOfThreads()
    {
        let mut thread_pool = ThreadPool::<i32, i32>::new(2).unwrap();
        assert_eq!(2, thread_pool.get_num_threads());
    
        thread_pool = ThreadPool::<i32, i32>::new(1).unwrap();
        assert_eq!(1, thread_pool.get_num_threads());

        thread_pool = ThreadPool::<i32, i32>::new(8).unwrap();
        assert_eq!(8, thread_pool.get_num_threads());
    }

    #[test]
    fn TestThreadPoolInvalidNumberOfThreadsError()
    {
        fn task(arg: i32) -> i32
        {
            return arg;
        }

        let thread_pool = ThreadPool::<i32, i32>::new(0);

        // err() -> Transforms the Result to an Option
        // The unwrap is an Option method that does not require Debug
        assert_eq!(TPErrors::InvalidNumberOfThreads, thread_pool.err().unwrap() );
    }

    #[test]
    fn TestThreadPoolThreadIsFullError()
    {
        fn task(arg: i32) -> i32
        {
            return arg;
        }

        let mut thread_pool = ThreadPool::<i32, i32>::new(1).unwrap();

        // The first run should be ok
        assert_eq!(Ok(()), thread_pool.execute(0, 1, task));


        // Since I did not get the results back, the second run should fail
        assert_eq!(Err(TPErrors::ThreadAlreadyHasATask), thread_pool.execute(0, 1, task));
    }

    /*#[test]
    fn TestThreadPool_ScopedThreads()
    {
        fn task(arg: (&mut [u64], usize)) -> usize
        {
            println!("{:?}", arg.0);
            arg.0.fill(arg.1 as u64);
            return arg.0.len();
        }

        let mut thread_pool = ThreadPool::new(8).unwrap();

        let var: i32 = 0;

        let mut vector_01 = vec![0; 25];
        let mut vector_02 = vec![1; 30];
        let mut vector_03 = vec![2; 25];
        let mut vector_04 = vec![3; 20];

        let mut input = vec![
            (vector_01.as_mut_slice(), 10),
            (vector_02.as_mut_slice(), 11),
            (vector_03.as_mut_slice(), 12),
            (vector_04.as_mut_slice(), 13)
        ];

        //let result = scope_execute!((&mut [u64], usize), usize, input, task, thread_pool);
        let result = thread_pool.scope_execute(input, task);

        println!("{:?}", result);
        println!("{:?}", vector_01);
        println!("{:?}", vector_02);
        println!("{:?}", vector_03);
        println!("{:?}", vector_04);

        assert!(false);
    }*/

    #[test]
    fn TestThreadPool_ScopedThreads()
    {
        fn task(arg: Vec<&u64>) -> ()
        {
            println!("Thread out: {:?}", arg[0]);
        }

        let mut thread_pool = ThreadPool::<Vec<&u64>, ()>::new(8).unwrap();

        let var_01: u64 = 0;

        let mut input = vec![
            vec![&var_01, &(10 as u64)],
        ];

        //let result = thread_pool.scope_execute(input, task);
        let result = scope_execute!(Vec<&u64>, (), input, task, thread_pool);
        println!("{:?}", result);

        assert!(false);
    }

    #[test]
    fn TestThreadPool_ScopedThreads02()
    {
        fn task(arg: (&u64, u64)) -> ()
        {
            println!("Thread out: {:?}", arg.0);
        }

        let mut thread_pool = ThreadPool::<(&u64, u64), ()>::new(8).unwrap();

        let var_01 = 0;

        let input = vec![
            (&var_01, 10),
        ];

        //let result = test_input(input, task);
        let result = thread_pool.scope_execute::<(&u64, u64)>(input, task);
        //let result = scope_execute!((&u64, u64), (), input, task, thread_pool);
        println!("{:?}", result);

        assert!(false);
    }
}