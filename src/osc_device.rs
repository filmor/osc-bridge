use crate::codec::OscCodec;
use futures::{SinkExt, StreamExt};
use rosc::{OscMessage, OscPacket, OscType};
use std::{net::SocketAddr, pin::Pin, sync::Arc};
use tokio::net::UdpSocket;
use tokio::sync::{Mutex, mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender}};
use tokio::task::JoinHandle;
use tokio_util::udp::UdpFramed;

pub struct OscDevice {
    //    source: Pin<Arc<dyn Stream<Item = OscPacket>>>,
    //    sink: Arc<Mutex<Pin<Box<dyn Sink<OscPacket, Error = std::io::Error>>>>>,
    _source_task: Pin<Box<JoinHandle<()>>>,
    source_recv: Arc<Mutex<UnboundedReceiver<OscPacket>>>,
    _sink_task: Pin<Box<JoinHandle<()>>>,
    sink_send: UnboundedSender<OscPacket>,
}

impl OscDevice {
    pub fn new(socket: UdpSocket, dest: SocketAddr) -> Self {
        let framed = UdpFramed::new(socket, OscCodec::new());
        let (mut sink, mut source) = framed.split();

        let (sink_send, mut sink_recv) = unbounded_channel();
        let (source_send, source_recv) = unbounded_channel();

        let source_task = tokio::spawn(async move {
            while let Some(res) = source.next().await {
                if let Ok((packet, _addr)) = res {
                    if let Err(_) = source_send.send(packet) {
                        return;
                    }
                }
            }
        });

        let sink_task = tokio::spawn(async move {
            while let Some(res) = sink_recv.next().await {
                let _res = sink.send((res, dest)).await;
            }
        });

        OscDevice {
            _source_task: Box::pin(source_task),
            source_recv: Arc::new(Mutex::new(source_recv)),
            _sink_task: Box::pin(sink_task),
            sink_send,
        }
    }

    pub async fn send_msg(&self, addr: &str, args: Vec<OscType>) {
        let addr = addr.to_owned();
        let msg = OscMessage { addr, args };
        let msg = OscPacket::Message(msg);

        let _res = self.sink_send.send(msg);
    }

    pub async fn receive_msg(&self) -> Option<OscPacket> {
        self.source_recv.lock().await.next().await
    }
}
