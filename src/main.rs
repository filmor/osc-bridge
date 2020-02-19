mod codec;
mod sync;
mod osc_device;

use futures::{StreamExt, FutureExt};
use log;
use pretty_env_logger;
use rosc::OscType;

#[tokio::main]
async fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init_timed();

    let mut left_device: osc_device::OscDevice =
        Box::pin(osc_device::discover_xr18()).next().await.unwrap();

    let mut right_device: osc_device::OscDevice =
        Box::pin(osc_device::discover_xr18()).next().await.unwrap();

    // TODO: Renew subscriptions after ~9s
    // TODO: Synchronise value

    let left_channel = "08";
    let right_channel = "07";
    left_device
        .send_msg(
            "/subscribe",
            vec![OscType::String(format!("/ch/{}/mix/pan", left_channel))],
        )
        .await;

    right_device
        .send_msg(
            "/subscribe",
            vec![OscType::String(format!("/ch/{}/mix/pan", right_channel))],
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
                }
                r = right_device.receive_msg().fuse() => {
                    log::info!("Right message: {:?}", r);
                }
            );
        }
    });

    fut.await.unwrap();
}
