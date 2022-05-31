use clap::Parser;
use nix::sys::socket::{recvmmsg, MsgFlags, RecvMmsgData, RecvMsg, SockaddrStorage};
use socket2::{Domain, Socket, Type};
use tokio::io::{Error, ErrorKind, Interest};
use tokio::net::UdpSocket;

use std::io::IoSliceMut;
use std::net::SocketAddr;
use std::os::unix::io::AsRawFd;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, Instant};

#[derive(Parser)]
struct Args {
    #[clap(short, default_value_t = 9000)]
    port: u16,
}

async fn recv<'a>(port: u16, count: Arc<AtomicUsize>) {
    let socket = Socket::new(Domain::IPV4, Type::DGRAM, None).unwrap();
    let address = SocketAddr::new("0.0.0.0".parse().unwrap(), port);
    socket.set_reuse_port(true).unwrap();
    socket.set_recv_buffer_size(1_000_000).unwrap();
    socket.set_nonblocking(true).unwrap();
    socket.bind(&address.into()).unwrap();
    let socket: UdpSocket = UdpSocket::from_std(socket.into()).unwrap();

    let mut last = Instant::now();

    let mut local_count = 0;

    loop {
        socket.readable().await.unwrap();
        let npkts = socket
            .try_io(Interest::READABLE, || {
                let sock_fd = socket.as_raw_fd();
                let mut buffers = [[0; 2048]; 100];
                let mut recv_mesg_data: Vec<RecvMmsgData<_>> = buffers
                    .iter_mut()
                    .map(|b| RecvMmsgData {
                        iov: [IoSliceMut::new(&mut b[..])],
                        cmsg_buffer: None,
                    })
                    .collect();
                let nptks: Vec<RecvMsg<'_, SockaddrStorage>> =
                    recvmmsg(sock_fd, &mut recv_mesg_data, MsgFlags::MSG_DONTWAIT, None).map_err(
                        |err| {
                            if err == nix::errno::Errno::EWOULDBLOCK {
                                return Error::new(ErrorKind::WouldBlock, "recvmmsg would block");
                            }
                            Error::new(ErrorKind::Other, err)
                        },
                    )?;
                Ok(nptks.len())
            })
            .unwrap_or(0);

        // let (size, _) = socket.recv_from(&mut buffer).await.unwrap();
        local_count += npkts;
        count.fetch_add(npkts, Ordering::SeqCst);

        if last.elapsed() > Duration::new(1, 0) {
            last = Instant::now();

            println!(
                "\treceived {} packets via {}",
                local_count,
                socket.as_raw_fd()
            );
        }
    }
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    let port = args.port;
    let count = Arc::new(AtomicUsize::new(0));

    std::thread::spawn({
        let count = count.clone();
        move || loop {
            sleep(Duration::from_secs(1));
            println!("Received {} packets", count.load(Ordering::SeqCst));
        }
    });

    tokio::try_join!(
        tokio::spawn({
            let count = count.clone();
            async move { recv(port, count).await }
        }),
        tokio::spawn({
            let count = count.clone();
            async move { recv(port, count).await }
        }),
    )
    .unwrap();
}
