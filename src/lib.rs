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

pub struct JoinHandle<T> {
    task: async_task::Task<T>,
}

impl<T> JoinHandle<T> {
    pub fn new(task: async_task::Task<T>) -> Self {
        Self { task }
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = T;

    fn poll(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        std::pin::Pin::new(&mut self.task).poll(cx)
    }
}

pub fn spawn<F, T>(f: F) -> JoinHandle<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    eprintln!("[runtime] Spawning new task...");
    let (runnable, task) = async_task::spawn(f, schedule);
    schedule(runnable);
    JoinHandle::new(task)
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
