use crate::codec::OscCodec;
use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::{SinkExt, StreamExt};
use rosc::{OscMessage, OscPacket, OscType};
use std::{net::SocketAddr, pin::Pin};
use tokio::net::UdpSocket;

use tokio::task::JoinHandle;
use tokio_util::udp::UdpFramed;

pub struct OscDevice {
    //    source: Pin<Arc<dyn Stream<Item = OscPacket>>>,
    //    sink: Arc<Mutex<Pin<Box<dyn Sink<OscPacket, Error = std::io::Error>>>>>,
    connected: bool,
    socket: Option<UdpSocket>,
    dest: SocketAddr,
    source_task: Option<Pin<Box<JoinHandle<()>>>>,
    sink_task: Option<Pin<Box<JoinHandle<()>>>>,
}

impl OscDevice {
    pub fn new(socket: UdpSocket, dest: SocketAddr) -> Self {
        OscDevice {
            connected: false,
            socket: Some(socket),
            dest,
            source_task: None,
            sink_task: None,
        }
    }

    pub fn connect(&mut self) -> (OscSender, UnboundedReceiver<OscPacket>) {
        if self.connected {
            panic!("Already connected")
        }

        let socket = self.socket.take().unwrap();

        let framed = UdpFramed::new(socket, OscCodec::new());
        let (mut sink, mut source) = framed.split();

        let (sink_send, mut sink_recv) = unbounded();
        let (mut source_send, source_recv) = unbounded();

        let source_task = tokio::spawn(async move {
            while let Some(res) = source.next().await {
                if let Ok((packet, _addr)) = res {
                    let _res = source_send.send(packet).await;
                }
            }
        });

        let dest = self.dest.clone();

        let sink_task = tokio::spawn(async move {
            while let Some(res) = sink_recv.next().await {
                let _res = sink.send((res, dest)).await;
            }
        });

        self.source_task = Some(Box::pin(source_task));
        self.sink_task = Some(Box::pin(sink_task));

        self.connected = true;

        (OscSender::new(sink_send), source_recv)
    }

    /* pub async fn send_msg(&mut self, addr: &str, args: Vec<OscType>) {
        let addr = addr.to_owned();
        let msg = OscMessage { addr, args };
        let msg = OscPacket::Message(msg);

        let _res = self.sink_send.send(msg);
    }*/
}

pub struct OscSender {
    inner: UnboundedSender<OscPacket>,
}

impl OscSender {
    fn new(inner: UnboundedSender<OscPacket>) -> Self {
        Self { inner }
    }

    pub async fn send_msg(&mut self, addr: &str, args: Vec<OscType>) {
        let addr = addr.to_owned();
        let msg = OscMessage { addr, args };
        let msg = OscPacket::Message(msg);

        let _res = self.inner.send(msg).await;
    }
}
