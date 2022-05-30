use socket2::{Domain, Socket, Type};
use tokio::net::UdpSocket;

use nix::sys::socket::{sendmmsg, MsgFlags, SendMmsgData, SockaddrIn};
use std::io::IoSlice;
use std::net::{SocketAddr, SocketAddrV4};
use std::os::unix::io::AsRawFd;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

async fn send(nb_packets: Arc<AtomicUsize>) {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, None).unwrap();
    let address: SocketAddr = "0.0.0.0:9000".parse().unwrap();
    socket.set_reuse_port(true).unwrap();
    socket.set_send_buffer_size(1_000_000).unwrap();
    // println!("SND BUFFER SIZE {}", socket.send_buffer_size().unwrap());
    socket.bind(&address.into()).unwrap();
    let socket: UdpSocket = UdpSocket::from_std(socket.into()).unwrap();
    let sock_fd = socket.as_raw_fd();

    let dest: SocketAddrV4 = "127.0.0.1:9001".parse().unwrap();
    let dest: SockaddrIn = dest.into();

    let buffers: Vec<SendMmsgData<_, _, _>> = (0..30)
        .map(|_| SendMmsgData {
            iov: [IoSlice::new(b"Hello World!")],
            cmsgs: &[],
            addr: dest.into(),
            _lt: Default::default(),
        })
        .collect();

    let mut last = Instant::now();
    let mut packets = 0;

    for _ in 0..100_000 {
        socket.writable().await.unwrap();
        let npkts = sendmmsg(sock_fd, &buffers, MsgFlags::MSG_DONTWAIT)
            .unwrap()
            .into_iter()
            .filter(|x| *x > 0)
            .count();

        packets += npkts;
        nb_packets.fetch_add(npkts, Ordering::SeqCst);

        if last.elapsed() > Duration::from_secs(1) {
            last = Instant::now();

            println!("\tSent {} packets to {}", packets, sock_fd);
        }
    }
}

#[tokio::main]
async fn main() {
    let nb_packets = Arc::new(AtomicUsize::new(0));

    let spawn = || {
        tokio::spawn({
            let nb_packets = nb_packets.clone();
            async move { send(nb_packets).await }
        })
    };

    tokio::try_join!(
        tokio::spawn({
            let nb_packets = nb_packets.clone();
            async move {
                loop {
                    tokio::time::sleep(Duration::from_secs(1)).await;
                    println!("Sent {} packets", nb_packets.load(Ordering::SeqCst));
                }
            }
        }),
        spawn(),
        spawn(),
        spawn(),
        spawn(),
        spawn(),
        spawn(),
    ).unwrap();
}
