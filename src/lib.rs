use std::{future::Future, sync::Mutex};

use async_task::Runnable;

static GLOBAL_QUEUE: Mutex<Vec<Runnable>> = Mutex::new(Vec::new());

pub struct Runtime {}

impl Runtime {
    pub fn new() -> Self {
        Runtime {}
    }

    pub fn block_on<T>(&self, f: impl Future<Output = T> + Send + 'static) {
        let wrapped_fut = async move {
            f.await;
        };
        let (runnable, task) = async_task::spawn(wrapped_fut, schedule);
        schedule(runnable);

        let t = std::thread::spawn(move || execute_from_queue(task));
        t.join().unwrap();
    }
}

pub fn spawn<F, T>(f: F) -> impl Future<Output = T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    eprintln!("[runtime] Spawning new task...");
    let (runnable, task) = async_task::spawn(f, schedule);
    schedule(runnable);
    task
}

fn schedule(runnable: Runnable) {
    eprintln!("[runtime] Scheduling task...");
    GLOBAL_QUEUE.lock().unwrap().push(runnable);
}

fn execute_from_queue<T, M>(task: async_task::Task<T, M>) {
    loop {
        eprintln!("[runtime] Fetching tasks...");
        let opt_runnable = GLOBAL_QUEUE.lock().unwrap().pop();
        if let Some(runnable) = opt_runnable {
            eprintln!("[runtime] Running task...");
            runnable.run();
            if task.is_finished() {
                break;
            }
        } else {
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
}
