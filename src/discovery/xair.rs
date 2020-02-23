use crate::osc_device::OscDevice;
use futures::stream::{FuturesUnordered, Stream, StreamExt};
use get_if_addrs::{get_if_addrs, IfAddr};
use rosc::decoder;
use std::io;
use std::net::Ipv4Addr;
use tokio::net::UdpSocket;

pub fn discover_xair() -> impl Stream<Item = OscDevice> {
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
