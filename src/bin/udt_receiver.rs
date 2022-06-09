use std::time::{Duration, Instant};
use tokio_udt::UdtListener;

use tokio::io::AsyncReadExt;

#[tokio::main]
async fn main() {
    let listener = UdtListener::bind("0.0.0.0:9000".parse().unwrap(), 10)
        .await
        .unwrap();

    println!("Waiting for connection...");

    let (addr, mut connection) = listener.accept().await.unwrap();

    println!("Accepted connection from {}", addr);

    let mut buffer = Vec::with_capacity(20_000_000);
    let mut last = Instant::now();
    let mut bytes = 0;

    loop {
        let size = connection.read_buf(&mut buffer).await.unwrap();
        bytes += size;

        if last.elapsed() > Duration::new(1, 0) {
            last = Instant::now();
            println!("Received {} MB", bytes as f64 / 1e6);
        }

        if buffer.len() >= 10_000_000 {
            buffer.clear();
        }
    }
}