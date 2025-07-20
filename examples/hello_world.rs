use gert::Runtime;

fn main() {
    let (tx, rx) = async_channel::unbounded();
    let t = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(1));
        eprintln!("[main] Sending message...");
        tx.send_blocking("Hello, man!").unwrap();
    });
    let runtime = Runtime::new();
    runtime.block_on(async move {
        eprintln!("[main] Hello, world!");
        let response = rx.recv().await.unwrap();
        eprintln!("[main] Received: {response}");
    });
    t.join().unwrap();
}
