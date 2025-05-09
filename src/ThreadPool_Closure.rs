// This should be a crate in the future

/*
# Considerations

    ## Moving closure API

    It is bound to be slower, because moving closures requires FnOnce() or FnOnce() and they require dyn (dinamic dispatch). It might also require Box to be able to store the pointer it into a struct. I have also made tests to verify if it was slower and I was able to confirm it (slightly slower).

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
use std::sync::Arc;
use std::boxed::Box;

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
struct TaskTP<RETURN_TYPE: 'static + Send + Sync>
{
    function_ptr: Option< Box<dyn FnOnce() -> RETURN_TYPE> >,
    exit: bool // Used when terminating the pool instance
}

impl<RETURN_TYPE: Send + Sync> TaskTP<RETURN_TYPE>
{
    fn new(function: Box<dyn FnOnce() -> RETURN_TYPE>) -> TaskTP<RETURN_TYPE>
    {
        return TaskTP
        {
            function_ptr: Some(function),
            exit: false
        };
    }

    // Creates an empty task to inform the worker thread that it can exit now
    fn exit() -> TaskTP<RETURN_TYPE>
    {
        return TaskTP
        {
            function_ptr: None,
            exit: true
        };
    }

    // Borrow the function pointer - read-only
    fn get_func_ptr(&self) -> Result< &Box<dyn FnOnce() -> RETURN_TYPE>, String>
    {
        match &self.function_ptr
        {
            Some(func) => return Ok(func),
            None => Err("No function present".to_string()),
        }
    }

    fn take_func_ptr(&mut self) -> Result< Box<dyn FnOnce() -> RETURN_TYPE>, String>
    {
        let closure =  mem::take(&mut self.function_ptr);

        match closure
        {
            Some(func) => return Ok(func),
            None => Err("No function present".to_string()),
        }
    }
}

// Tell the compiler we can send this type between threads
unsafe impl<RETURN_TYPE: Send + Sync> Send for TaskTP<RETURN_TYPE> {}


// Thread from Thread Pool
// It represents a single thread in the pool
struct ThreadTP<RETURN_TYPE: 'static + Send + Sync>
{
    handle: Option< thread::JoinHandle<Result<(), String>> >,
    assigned: bool, // Did main send a task?

    // Main sends tasks, worker receives them
    task_queue_sender: mpsc::Sender< TaskTP<RETURN_TYPE> >,
    task_queue_receiver: Option< mpsc::Receiver< TaskTP<RETURN_TYPE>> >,

    // Worker sends results, main receives them
    result_queue_sender: Option< mpsc::Sender<RETURN_TYPE> >,
    result_queue_receiver: mpsc::Receiver<RETURN_TYPE>
}

impl<RETURN_TYPE: 'static + Send + Sync> ThreadTP<RETURN_TYPE>
{
    fn new() -> ThreadTP<RETURN_TYPE>
    {
        let (task_s, task_r) = mpsc::channel::< TaskTP<RETURN_TYPE> >();
        let (result_s, result_r) = mpsc::channel::<RETURN_TYPE>();

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
    fn take_worker_task_receiver(&mut self) -> mpsc::Receiver< TaskTP<RETURN_TYPE>>
    {
        let receiver: Option< mpsc::Receiver< TaskTP<RETURN_TYPE>> > =  mem::take(&mut self.task_queue_receiver);
        
        match receiver
        {
            Some(recv) => return recv,
            None => todo!()
        }
    }

    // Takes ownsership of the sender of the result queue, leaving None
    // Main doesn't send any results, so the worker can have full ownsership 
    fn take_worker_result_sender(&mut self) -> mpsc::Sender<RETURN_TYPE>
    {
        let sender: Option< mpsc::Sender<RETURN_TYPE> > = mem::take(&mut self.result_queue_sender);
        
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
pub struct ThreadPool<RETURN_TYPE: 'static + Send + Sync>
{
    thread_list: Vec< ThreadTP<RETURN_TYPE> >,
}

// Send + 'static is required by thread::spawn -> they don't cause mem leaks as the underlying data gets deallocated
impl<RETURN_TYPE: 'static + Send + Sync> ThreadPool<RETURN_TYPE>
{
    pub fn new(num_threads: usize) -> Result<ThreadPool<RETURN_TYPE>, TPErrors>
    {
        // Function fails with an invalid number of threads
        // In other words, you must create a pool with at least 1 thread
        // TODO: alert the user that values <=0 should make this function fail - would a Result be better?
        if num_threads < 1
        {
            return Err(TPErrors::InvalidNumberOfThreads);
        }

        // Thread list - holds the handles to each thread
        let mut new_thread_list: Vec< ThreadTP<RETURN_TYPE> > = Vec::new();

        for idx in 0..num_threads
        {
            // Thread - contains general information about the thread
            let mut thread = ThreadTP::<RETURN_TYPE>::new();

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
                        let task_box: Box<dyn FnOnce() -> RETURN_TYPE> = task.take_func_ptr().unwrap();
                        let results = task_box();

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


    pub fn execute(&mut self, thread_id: usize, task: Box<dyn FnOnce() -> RETURN_TYPE>) -> Result<(), TPErrors>
    {

        // One should not send tasks to already assinged threads
        if self.thread_list[thread_id].assigned == true
        {
            return Err(TPErrors::ThreadAlreadyHasATask);
        }

        // Sends a task to the thread
        self.thread_list[thread_id].task_queue_sender.send( TaskTP::new(task) );

        // Keep track that it has been started by main
        self.thread_list[thread_id].assigned = true;

        return Ok(());
    }

    pub fn wait_all(&mut self) -> Result<Vec<RETURN_TYPE>, String>
    {
        let mut results: Vec<RETURN_TYPE> = Vec::new();

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

    pub fn scope_execute(&mut self, mut tasks: Vec< Box<dyn FnOnce() -> RETURN_TYPE> >) -> Vec<RETURN_TYPE>
    {
        for t_idx in 0..tasks.len()
        {
            println!("{}", std::any::type_name::<RETURN_TYPE>());
            print_type_of(&tasks);

            let task = tasks.remove(0);

            // Check for any errors: in this case panic, because the error might not be recoverable (and can't resend tasks if something did go wrong)
            self.execute(t_idx, task).unwrap();
        }
    
        // Wait for all threads to finish and return the results
        self.wait_all().unwrap()
    }
}

impl<RETURN_TYPE: 'static + Send + Sync> Drop for ThreadPool<RETURN_TYPE>
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
    use crate::ThreadPool_Closure::*;

    /*
    #[test]
    fn TestThreadPool_Closure_Execute()
    {
        let mut thread_pool = ThreadPool::new(8).unwrap();

        {
            let mut vector = vec![0, 1];

            thread_pool.execute(0, Box::new(move||{
                loop
                {
                    println!("{:?}", &vector);
                    thread::sleep( std::time::Duration::from_millis(1) );
                }
            }));

            println!("{:?}", vector);
        }

        thread::sleep( std::time::Duration::from_millis(2000) );
        assert!(false);
    }*/

    #[test]
    fn TestThreadPool_Closure_ScopedThreads()
    {

        let mut thread_pool = ThreadPool::new(8).unwrap();

        let mut var_01: u64 = 66;
        let mut var_02: u64 = 0;
        let mut var_03: u64 = 0;
        let mut hello = vec!["h", "e", "l", "l", "o"];

        let list: Vec<u64> = vec![1, 2, 3];

        let mut input: Vec< Box<dyn FnOnce() -> u64> > = vec![
            Box::new(move||{ *(&var_01) }),
            Box::new(move||{ *(&var_02) }),
            Box::new(move||{ *(&var_03) }),
            Box::new(move||{ hello[..].fill("g"); 10 as u64 }),
        ];

        let result = thread_pool.scope_execute(input);
        println!("{:?}", result);
        println!("{:?}", hello);

        assert!(false);
    }

    /*
    #[test]
    fn TestThreadPool_ScopedThreads02()
    {
        fn task(arg: (&u64, u64)) -> ()
        {
            println!("Thread out: {:?}", arg.0);
        }

        let mut thread_pool = ThreadPool::<(&u64, u64), ()>::new(8).unwrap();

        let mut var_01 = 0;
        let mut var_02 = 0;
        let mut var_03 = 0;

        let mut input = vec![
            (&var_01, 10),
        ];

        //let result = test_input(input, task);
        let result = thread_pool.scope_execute::<(&u64, u64)>(input, task);
        //let result = scope_execute!((&u64, u64), (), input, task, thread_pool);
        println!("{:?}", result);

        assert!(false);
    }
    */
}