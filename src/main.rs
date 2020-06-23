mod codec;
mod discovery;
mod osc_device;
mod sync;

use crate::discovery::{connect_ds100, discover_xair};
use futures::StreamExt;
use log;
use osc_device::OscDevice;
use pretty_env_logger;
use rosc::{OscPacket::*, OscType};
use std::time::Duration;

#[tokio::main]
async fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init_timed();

    let mut left_device: OscDevice = Box::pin(discover_xair()).next().await.unwrap();
    let (mut left_send, mut left_recv) = left_device.connect();

    // 192.168.1.104 50000
    let ip_ds100 = std::net::Ipv4Addr::new(192, 168, 1, 104);
    let mut right_device: OscDevice = Box::pin(connect_ds100(ip_ds100)).next().await.unwrap();
    let (mut right_send, mut right_recv) = right_device.connect();

    // TODO: Synchronise value

    let mut resubscribe = tokio::time::interval(Duration::from_secs(9));

    let left_channel = "08";
    left_send
        .send_msg(
            "/subscribe",
            vec![OscType::String(format!("/ch/{}/mix/pan", left_channel))],
        )
        .await;

    log::info!("Sent subscribe message");

    let mut sync = sync::Sync::new();

    sync.update(sync::Left, 0.0);
    sync.update(sync::Right, 0.0);

    let fut = tokio::spawn(async move {
        loop {
            tokio::select!(
                l = left_recv.next() => {
                    log::info!("Left message: {:?}", l);

                    if let Some(Message(msg)) = l {
                        // if !msg.addr.starts_with("/ch") { return; }
                        let var = msg.args[0].clone();
                        right_send.send_msg("/dbaudio1/coordinatemapping/source_position_x/1/1", vec![var]).await;
                    }

                    // /dbaudio1/positioning/source_position_x/1
            // /dbaudio1/matrixinput/reverbsendgain/ Kanal float
                }
                r = right_recv.next() => {
                    log::info!("Right message: {:?}", r);
                }
                _ = resubscribe.next() => {
                    // log::info!("Timer");
                    left_send
                        .send_msg(
                            "/subscribe",
                            vec![OscType::String(format!("/ch/{}/mix/pan", left_channel))],
                        )
                        .await;
                }
            );
        }
    });

    fut.await.unwrap();
}
