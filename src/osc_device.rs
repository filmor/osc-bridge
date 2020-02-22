use crate::codec::OscCodec;
use futures::stream::{FuturesUnordered, StreamExt};
use futures::{SinkExt, Stream};
use get_if_addrs::{get_if_addrs, IfAddr};
use rosc::{decoder, OscMessage, OscPacket, OscType};
use std::io;
use std::net::{Ipv4Addr, SocketAddr};
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

pub fn discover_xr18() -> impl Stream<Item = OscDevice> {
    let futures = FuturesUnordered::new();

    if let Ok(if_addrs) = get_if_addrs() {
        let bc_addrs = if_addrs
            .iter()
            .filter_map(|if_addr| match if_addr.addr.clone() {
                IfAddr::V4(addr) => {
                    if let Some(bc) = addr.broadcast {
                        Some((addr.ip, bc))
                    } else {
                        None
                    }
                }
                _ => None,
            });

        for (ip, bc_addr) in bc_addrs {
            futures.push(tokio::spawn(request_initial_info(ip, bc_addr)));
        }
    }

    futures.filter_map(|f| async move {
        match f {
            Ok(Ok(x)) => Some(x),
            _ => None,
        }
    })
}

pub fn connect_ds100(addr: Ipv4Addr) -> impl Stream<Item = OscDevice> {
    let futures = FuturesUnordered::new();

    if let Ok(if_addrs) = get_if_addrs() {
        for ip in if_addrs.iter().filter_map(|if_addr| match if_addr.addr.clone() {
            IfAddr::V4(addr) => Some(addr.ip),
            _ => None,
        }) {
            futures.push(tokio::spawn(request_ds100_device_name(ip, addr)));
        }
    }

    futures.filter_map(|f| async move {
        match f {
            Ok(Some(x)) => Some(x),
            _ => None,
        }
    })
}

async fn request_initial_info(addr: Ipv4Addr, bc_addr: Ipv4Addr) -> Result<OscDevice, io::Error> {
    let port = 10024; // 10023 for X32

    let buf = b"/xinfo";

    let mut socket: UdpSocket = UdpSocket::bind((addr, 0)).await?;
    log::info!("Listening on {}", socket.local_addr().unwrap());

    socket.set_broadcast(true)?;

    log::info!("Sending /xinfo broadcast to {}:{}", bc_addr, port);

    socket.send_to(buf, (bc_addr, port)).await?;

    let mut buf = vec![0; 256];
    let (_, src) = socket.recv_from(&mut buf).await?;

    let res = decoder::decode(&buf).unwrap();
    log::info!("Message: {:?}", res);

    socket.connect(src).await?;
    socket.set_broadcast(false)?;

    Ok(OscDevice::new(socket, src))
}

async fn request_ds100_device_name(addr: Ipv4Addr, out: Ipv4Addr) -> Option<OscDevice> {
    let send = 50010;
    let recv = 50011;

    log::info!("Sending devicename request to {:?} from {:?}", out, addr);

    let socket = UdpSocket::bind((addr, recv)).await.ok()?;

    let mut res = OscDevice::new(socket, (out, send).into());
    res.send_msg("/dbaudio1/settings/devicename", vec![]).await;

    let received = res.receive_msg().await;
    received.map(|msg| {
        log::info!("Got answer: {:?}", msg);
        res
    })
}
