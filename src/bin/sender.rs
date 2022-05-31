use clap::Parser;
use socket2::{Domain, Socket, Type};
use tokio::io::Interest;
use tokio::io::{Error, ErrorKind};
use tokio::net::UdpSocket;

use nix::sys::socket::{sendmmsg, MsgFlags, SendMmsgData, SockaddrIn};
use std::io::IoSlice;
use std::net::SocketAddrV4;
use std::os::unix::io::AsRawFd;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Parser)]
struct Args {
    dest_ip: String,

    #[clap(short, default_value_t = 9000)]
    port: u16,
}

async fn send(dest: SocketAddrV4, nb_packets: Arc<AtomicUsize>) {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, None).unwrap();
    socket.set_reuse_port(true).unwrap();
    socket.set_send_buffer_size(1_000_000).unwrap();
    socket.set_nonblocking(true).unwrap();
    let socket: UdpSocket = UdpSocket::from_std(socket.into()).unwrap();

    let dest: SockaddrIn = dest.into();

    let buffers: Vec<SendMmsgData<_, _, _>> = (0..200)
        .map(|_| SendMmsgData {
            iov: [IoSlice::new(b"Hello World!")],
            cmsgs: &[],
            addr: dest.into(),
            _lt: Default::default(),
        })
        .collect();

    let mut last = Instant::now();
    let mut packets = 0;

    for _ in 0..50_000 {
        socket.writable().await.unwrap();
        let npkts = socket
            .try_io(Interest::WRITABLE, || {
                let sock_fd = socket.as_raw_fd();
                let npkts = sendmmsg(sock_fd, &buffers, MsgFlags::MSG_DONTWAIT)
                    .map_err(|err| {
                        if err == nix::errno::Errno::EWOULDBLOCK {
                            return Error::new(ErrorKind::WouldBlock, "sendmmsg would block");
                        }
                        Error::new(ErrorKind::Other, format!("{}", err))
                    })?
                    .into_iter()
                    .filter(|x| *x > 0)
                    .count();
                Ok(npkts)
            })
            .unwrap_or(0);

        packets += npkts;
        nb_packets.fetch_add(npkts, Ordering::SeqCst);

        if last.elapsed() > Duration::from_secs(1) {
            last = Instant::now();
            let sock_fd = socket.as_raw_fd();
            println!("\tSent {} packets via {}", packets, sock_fd);
        }
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let dest = SocketAddrV4::new(args.dest_ip.parse().unwrap(), args.port);

    let nb_packets = Arc::new(AtomicUsize::new(0));

    let spawn = || {
        tokio::spawn({
            let nb_packets = nb_packets.clone();
            async move { send(dest, nb_packets).await }
        })
    };

    std::thread::spawn({
        let nb_packets = nb_packets.clone();
        move || loop {
            std::thread::sleep(Duration::from_secs(1));
            println!("Sent {} packets", nb_packets.load(Ordering::SeqCst));
        }
    });

    tokio::try_join!(
        spawn(),
        spawn(),
    ).unwrap();

    println!("Sent {} packets", nb_packets.load(Ordering::SeqCst));
}
