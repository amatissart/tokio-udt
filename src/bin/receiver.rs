use socket2::{Domain, Socket, Type};
use tokio::net::UdpSocket;

use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use std::thread::sleep;
use std::os::unix::io::AsRawFd;



async fn recv(count: Arc<AtomicUsize>) -> tokio::task::JoinHandle<()> {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, None).unwrap();
    let address: SocketAddr = "0.0.0.0:9001".parse().unwrap();
    socket.set_reuse_port(true).unwrap();
    // socket.set_recv_buffer_size(1_000_000).unwrap();
    socket.bind(&address.into()).unwrap();
    let socket: UdpSocket = UdpSocket::from_std(socket.into()).unwrap();

    let mut buffer = [0; 2048];
    let mut last = Instant::now();
    let raw_fd = socket.as_raw_fd();

    loop {
        let (size, _) = socket.recv_from(&mut buffer).await.unwrap();
        count.fetch_add(1, Ordering::SeqCst);

        if last.elapsed() > Duration::new(1, 0) {
            last = Instant::now();

            println!("\t(size is {}) from {}", size, socket.as_raw_fd());
        }
    }
}

#[tokio::main]
async fn main() {
    let count = Arc::new(AtomicUsize::new(0));

    tokio::try_join!(
        tokio::spawn({
            let count = count.clone();
            async move {
                loop {
                    sleep(Duration::from_secs(1));
                    println!("Received {} packets", count.load(Ordering::SeqCst));
                }
            }
        }),
        tokio::spawn({
            let count = count.clone();
            async move { recv(count).await }
        }),
        // tokio::spawn({
        //     let count = count.clone();
        //     async move { recv(count).await }
        // }),
        // tokio::spawn({
        //     let count = count.clone();
        //     async move { recv(count).await }
        // }),
    ).unwrap();
}
