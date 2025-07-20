fn main() {
    let start = std::time::Instant::now();
    gert::Runtime::new().block_on(async move {
        let (tx, rx) = async_channel::unbounded();
        eprintln!("[main] Hello, world!");
        let handle = gert::spawn(async move { send_message(tx).await });
        std::thread::sleep(std::time::Duration::from_secs(1));
        let response = rx.recv().await.unwrap();
        eprintln!("[main] Received: {response}");
        handle.await;
    });
    let end = std::time::Instant::now();
    eprintln!("[main] Execution time: {:?}", end.duration_since(start));
}

async fn send_message(tx: async_channel::Sender<&'static str>) {
    std::thread::sleep(std::time::Duration::from_secs(1));
    eprintln!("[send_message] Sending message...");
    tx.send("Hello, man!").await.unwrap();
}
