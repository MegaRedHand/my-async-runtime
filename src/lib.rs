use std::{
    future::Future,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
};

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

        let mut executors = vec![];
        let mut progress = vec![];

        std::thread::scope(|s| {
            let mut handles = vec![];

            executors.push(Arc::new(Executor::new()));
            let executor = executors.last().unwrap().clone();
            progress.push(0);
            let jh = s.spawn(move || executor.execute_from_queue());
            handles.push(jh);

            loop {
                std::thread::sleep(std::time::Duration::from_millis(10));

                if task.is_finished() {
                    break;
                }
                if GLOBAL_QUEUE.lock().unwrap().is_empty() {
                    continue;
                }
                let mut blocked_executors = vec![];
                for (last_progress, executor) in progress.iter_mut().zip(&executors) {
                    let current_progress = executor.progress.load(Ordering::Acquire);
                    if current_progress > *last_progress {
                        // Progress has been made, so we can continue
                        *last_progress = current_progress;
                    } else {
                        blocked_executors.push(executor.clone());
                    }
                }
                for _executor in blocked_executors {
                    eprintln!("[runtime] Executor is blocked, spawning new executor...");
                    executors.push(Arc::new(Executor::new()));
                    let executor = executors.last().unwrap().clone();
                    progress.push(0);
                    let jh = s.spawn(move || executor.execute_from_queue());
                    handles.push(jh);
                }
            }
            eprintln!("[runtime] All tasks completed, shutting down executors...");
            executors.iter().for_each(|executor| {
                executor.is_finished.store(true, Ordering::Relaxed);
            });
        });
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
    eprintln!("[{:?}] Spawning new task...", std::thread::current().id());
    let (runnable, task) = async_task::spawn(f, schedule);
    schedule(runnable);
    JoinHandle::new(task)
}

fn schedule(runnable: Runnable) {
    eprintln!("[{:?}] Scheduling task...", std::thread::current().id());
    GLOBAL_QUEUE.lock().unwrap().push(runnable);
}

struct Executor {
    progress: AtomicU64,
    is_finished: AtomicBool,
}

impl Executor {
    fn new() -> Self {
        Executor {
            progress: AtomicU64::new(0),
            is_finished: AtomicBool::new(false),
        }
    }

    fn execute_from_queue(&self) {
        while !self.is_finished.load(Ordering::Relaxed) {
            eprintln!("[{:?}] Fetching tasks...", std::thread::current().id());
            let opt_runnable = GLOBAL_QUEUE.lock().unwrap().pop();
            if let Some(runnable) = opt_runnable {
                eprintln!("[{:?}] Running task...", std::thread::current().id());
                runnable.run();
            } else {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            self.progress.fetch_add(1, Ordering::AcqRel);
        }
    }
}
