use crate::codec::OscCodec;
use futures::stream::StreamExt;
use futures::SinkExt;
use rosc::{OscMessage, OscPacket, OscType};
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio_util::udp::UdpFramed;

pub struct OscDevice {
    framed: UdpFramed<OscCodec>,
    dest: SocketAddr,
}

impl OscDevice {
    pub fn new(socket: UdpSocket, dest: SocketAddr) -> Self {
        let framed = UdpFramed::new(socket, OscCodec::new());
        OscDevice { framed, dest }
    }

    pub async fn send_msg(&mut self, addr: &str, args: Vec<OscType>) {
        let addr = addr.to_owned();
        let msg = OscMessage { addr, args };
        let msg = OscPacket::Message(msg);

        let _res = self.framed.send((msg, self.dest)).await;
    }

    pub async fn receive_msg(&mut self) -> Option<OscPacket> {
        self.framed.next().await?.ok().map(|(packet, _addr)| packet)
    }
}
