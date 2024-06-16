// This should be a crate in the future

// TODO! Remove pub in substructures, they should be private
// TODO! Test a new API that accepts moving closures insted of the current one that unpacks a struct ARG
//      This should make the pool API closer to other Rust's thread APIs
//      It should also allow different types of arguments for different tasks in the same pool
// TODO! Investigate variadic fuctions API

/*
# Considerations

    ## Moving closure API

    It is bound to be slower, because moving closures requires Fn() or FnOnce() and they require dyn (dinamic dispatch). It might also require Box to be able to store the pointer it into a struct. I have also made tests to verify if it was slower and I was able to confirm it (slightly slower).
    
    Since I don't belive this style of API brings any benefits, moving closures won't be supported.
*/

// Implementation of a thread pool to reduce the need of recreating threads all of the time
// It reduces the cost of thread creation syscall

use std::sync::mpsc;
use std::thread;
use std::mem;


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
pub struct TaskTP<ARGS, RETURN_STRUCT>
{
    arguments_struct: Option<ARGS>,
    function_ptr: Option< fn(args: ARGS) -> RETURN_STRUCT >,
    exit: bool // Used when terminating the pool instance
}

impl<ARGS, RETURN_STRUCT> TaskTP<ARGS, RETURN_STRUCT>
{
    pub fn new(args: ARGS, function: fn(args: ARGS) -> RETURN_STRUCT) -> TaskTP<ARGS, RETURN_STRUCT>
    {
        return TaskTP
        {
            arguments_struct: Some(args),
            function_ptr: Some(function),
            exit: false
        };
    }

    // Creates an empty task to inform the worker thread that it can exit now
    pub fn exit() -> TaskTP<ARGS, RETURN_STRUCT>
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
pub struct ThreadTP<ARGS, RETURN_STRUCT>
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

// The actual pool, it controls all other substructures
pub struct ThreadPool<ARGS, RETURN_STRUCT>
{
    thread_list: Vec< ThreadTP<ARGS, RETURN_STRUCT> >,
}

// Send + 'static is required by thread::spawn -> they don't cause mem leaks as the underlying data gets deallocated
impl<ARGS: Send + 'static, RETURN_STRUCT: Send + 'static> ThreadPool<ARGS, RETURN_STRUCT>
{
    pub fn new(num_threads: usize) -> ThreadPool<ARGS, RETURN_STRUCT>
    {
        // Thread list - holds the handles to each thread
        let mut new_thread_list: Vec< ThreadTP<ARGS, RETURN_STRUCT> > = Vec::new();

        for idx in 0..num_threads
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

        return ThreadPool
        {
            thread_list: new_thread_list,
        };
    }


    pub fn execute(&mut self, thread_id: usize, args: ARGS, task: fn(args: ARGS) -> RETURN_STRUCT) -> Result<(), String>
    {

        // One should not send tasks to already assinged threads
        if self.thread_list[thread_id].assigned == true
        {
            return Err("Thread already has a task assigned to it".to_string());
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


// It is not possible to use barriers in this particular implementation, destroy might deadlock if only some threads were assigned work (pool of 10, only 5 have work)
// Picture this situation:
// some threads have some tasks --> blocked at barrier wait : to resume all other threads must rendevousz at barrier wait
// threads without tasks --> blocked at park : to resume we call unpark


// Unit test
#[cfg(test)]
mod tests
{
    use crate::ThreadPool::*;

    #[ignore]
    #[test]
    fn ThreadPoolTest()
    {
        fn task_func(args: i32) -> i32
        {
            println!("Hello from thread task");
            return args.clone();
        }

        let mut thread_list: Vec< ThreadTP<i32, i32> >= Vec::new();

        for idx in 0..5
        {
            let task = TaskTP::<i32, i32>::new(1, task_func);

            println!("Task Object \nArgs: {} \nFunc: {:?} \nExit: {}", task.get_args().unwrap(), task.get_func_ptr().unwrap(), task.exit);
            println!("\n\n\n");

            let mut thread = ThreadTP::<i32, i32>::new();

            println!("ThreadTP Object \nHandle: {:?} \nAssigned: {} \nTaskSender: {:?} \nTaskReceiver: {:?} \nResultSender: {:?} \nResultReceiver: {:?}",
            thread.handle,
            thread.assigned,
            thread.task_queue_sender,
            thread.task_queue_receiver,
            thread.result_queue_sender,
            thread.result_queue_receiver,
            );
            println!("\n\n\n");
            

            // Send the task
            thread.task_queue_sender.send(task);

            // Setup thread
            let thread_task_rcv = thread.take_worker_task_receiver();

            let thread_handle = thread::spawn(move|| -> Result<(), String>
                {
                    loop
                    {
                        //let received_task = task_r.recv().unwrap();
                        let mut received_task = thread_task_rcv.recv().unwrap();

                        if received_task.exit == true
                        {
                            return Ok(());
                        }

                        println!("Task Object \nArgs: {} \nFunc: {:?} \nExit: {}", received_task.get_args().unwrap(), received_task.get_func_ptr().unwrap(), received_task.exit);
                        println!("\n\n\n");

                        ( received_task.get_func_ptr().unwrap() )( received_task.take_args().unwrap() );
                    }
                });

            thread.handle = Some(thread_handle);

            thread_list.push(thread);
        }

        for idx in 0..5
        {
            let thread = thread_list.pop().unwrap();
            thread.task_queue_sender.send(TaskTP::<i32, i32>::exit());
            thread.handle.unwrap().join();
        }

        assert_eq!(false,true);
    }

    #[ignore]
    #[test]
    fn TestPool()
    {
        use::std::time;
        {
            let num_threads: usize = 10;
            let mut thread_pool = ThreadPool::<(i32, i32), i32>::new(num_threads);

            fn task(arg: (i32, i32)) -> i32
            {
                let (arg1, arg2) = arg; 
                println!("Hello from task! Args: {:?}", arg);
                return arg1;
            }

            for idx in 0..num_threads
            {
                thread_pool.execute(idx, (idx as i32, idx as i32), task);
            }

            let all_results = thread_pool.wait_all();
            println!("Results: {:?}", all_results);

            for idx in 0..num_threads
            {
                thread_pool.execute(idx, ((idx+num_threads-1) as i32, idx as i32), task);
            }

            let all_results = thread_pool.wait_all();
            println!("Results: {:?}", all_results);
        }

        thread::sleep(time::Duration::from_millis(1000));

        assert_eq!(false,true);
    }

    #[ignore]
    #[test]
    fn TestPoolArgsLeak()
    {
        use::std::time;
        {
            let mut thread_pool = ThreadPool::<Vec<u8>, i32>::new(1);

            fn task(arg: Vec<u8>) -> i32
            { 
                return 0;
            }

            loop
            {
                let vector: Vec<u8> = vec![1; 1*1024*1024];

                let _ = thread_pool.execute(0 as usize, vector, task);
                let all_results = thread_pool.wait_all();
            }

            thread::sleep(time::Duration::from_millis(10000));
        }
        assert_eq!(false,true);
    }


    #[ignore]
    #[test]
    fn TestPoolReturnLeak()
    {
        use::std::time;
        {
            let mut thread_pool = ThreadPool::<i32, Vec<u8>>::new(1);

            fn task(arg: i32) -> Vec<u8>
            { 
                let vector: Vec<u8> = vec![1; 1*1024*1024];
                return vector;
            }

            loop
            {
                let _ = thread_pool.execute(0 as usize, 1, task);
                let all_results = thread_pool.wait_all();
            }

            //thread::sleep(time::Duration::from_millis(10000));
        }
        assert_eq!(false,true);
    }

    #[ignore]
    #[test]
    fn TestPoolCreationLeak()
    {
        use::std::time;
        {
            fn task(arg: i32) -> i32
            { 
                return arg;
            }

            loop
            {
                let mut thread_pool = ThreadPool::<i32, i32>::new(1);
                let _ = thread_pool.execute(0 as usize, 1, task);
                let all_results = thread_pool.wait_all();
            }

            //thread::sleep(time::Duration::from_millis(10000));
        }
        assert_eq!(false,true);
    }

    #[test]
    fn TestPoolPerformanceMeasure()
    {
        use::std::time;
        {
            let num_tasks: usize = 1000000;

            fn task(arg: i32) -> i32
            { 
                return arg;
            }

            let mut thread_pool = ThreadPool::<i32, i32>::new(1);

            let mut now = time::Instant::now();
            for _ in 0..num_tasks
            {
                let _ = thread_pool.execute(0 as usize, 1, task);
                let all_results = thread_pool.wait_all();
            }
            let thread_pool_time_elapsed = now.elapsed().as_millis();
            println!("Thread pool time elapsed: {}ms", thread_pool_time_elapsed);
            println!("Thread pool, tasks p/ milisec: {}t/ms", num_tasks/thread_pool_time_elapsed as usize);

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

        }
        assert_eq!(false,true);
    }
}