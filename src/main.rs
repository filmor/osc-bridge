mod codec;
mod osc_device;

use get_if_addrs::{get_if_addrs, IfAddr};
use log;
use pretty_env_logger;
use rosc::*;
use tokio::net::UdpSocket;
use tokio::time::{Instant, interval_at};
use std::time::Duration;
use tokio_util::udp::UdpFramed;
use futures::{StreamExt, SinkExt};

#[tokio::main]
async fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init();

    let framed: osc_device::Framed = Box::pin(osc_device::discover_xr18()).next().await.unwrap();

    let channel = "08";
    /* framed.send(OscPacket::Message(
        OscMessage {
            addr: "/subscribe",
            args: [format!("/ch/{}/pan", channel)]
        }
    ));*/

    let iv = interval_at(Instant::now(), Duration::from_secs(9));
    let (mut sink, mut stream) = framed.split();

    loop {
        let p = stream.next().await;
        log::info!("Message: {:?}", p);
    }
}
