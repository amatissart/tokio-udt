use crate::socket::SocketType;
use crate::udt::{SocketRef, Udt};
use std::net::SocketAddr;
use tokio::io::Result;

pub struct UdtConnection {
    socket: SocketRef,
}

impl UdtConnection {
    pub(crate) fn new(socket: SocketRef) -> Self {
        Self { socket }
    }

    pub async fn connect(addr: SocketAddr) -> Result<Self> {
        let mut udt = Udt::get().write().await;
        let socket: &SocketRef = udt.new_socket(SocketType::Stream)?;
        {
            let mut socket = socket.write().await;
            socket.connect(addr).await?;
        }
        Ok(Self {
            socket: socket.clone(),
        })
    }

    pub async fn send(&self, msg: &[u8]) -> Result<()> {
        let mut socket = self.socket.write().await;
        socket.send(msg).await?;
        Ok(())
    }

    pub async fn recv(&self, buf: &mut [u8]) -> Result<usize> {
        let mut socket = self.socket.write().await;
        let nbytes = socket.recv(buf).await?;
        Ok(nbytes)
    }
}