mod codec;
mod discovery;
mod osc_device;
mod sync;

use futures::{FutureExt, StreamExt};
// use futures_util::pin_mut;
use crate::discovery::{connect_ds100, discover_xair};
use log;
use pretty_env_logger;
use rosc::{OscPacket::*, OscType};
use std::time::Duration;

#[tokio::main]
async fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init_timed();

    let mut left_device: osc_device::OscDevice = Box::pin(discover_xair()).next().await.unwrap();

    // 192.168.1.104 50000
    let ip_ds100 = std::net::Ipv4Addr::new(192, 168, 1, 104);
    let mut right_device = Box::pin(connect_ds100(ip_ds100)).next().await.unwrap();

    // TODO: Synchronise value

    let mut resubscribe = tokio::time::interval(Duration::from_secs(9));

    let left_channel = "08";
    left_device
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
            futures::select!(
                l = left_device.receive_msg().fuse() => {
                    log::info!("Left message: {:?}", l);

                    if let Some(Message(msg)) = l {
                        // if !msg.addr.starts_with("/ch") { return; }
                        let var = msg.args[0].clone();
                        right_device.send_msg("/dbaudio1/coordinatemapping/source_position_x/1/1", vec![var]).await;
                    }

                    // /dbaudio1/positioning/source_position_x/1
            // /dbaudio1/matrixinput/reverbsendgain/ Kanal float
                }
                r = right_device.receive_msg().fuse() => {
                    log::info!("Right message: {:?}", r);
                }
                _ = resubscribe.next().fuse() => {
                    // log::info!("Timer");
                    left_device
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
