use crate::osc_device::OscDevice;
use futures::stream::{FuturesUnordered, Stream, StreamExt};
use get_if_addrs::{get_if_addrs, IfAddr};
use std::net::Ipv4Addr;
use tokio::net::UdpSocket;

pub fn connect_ds100(addr: Ipv4Addr) -> impl Stream<Item = OscDevice> {
    let futures = FuturesUnordered::new();

    if let Ok(if_addrs) = get_if_addrs() {
        for ip in if_addrs
            .iter()
            .filter_map(|if_addr| match if_addr.addr.clone() {
                IfAddr::V4(addr) => Some(addr.ip),
                _ => None,
            })
        {
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
