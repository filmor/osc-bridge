mod codec;
mod sync;
mod osc_device;

use futures::StreamExt;
use log;
use pretty_env_logger;
use rosc::OscType;

#[tokio::main]
async fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init();

    let mut device: osc_device::OscDevice =
        Box::pin(osc_device::discover_xr18()).next().await.unwrap();

    let channel = "08";
    device
        .send_msg(
            "/subscribe",
            vec![OscType::String(format!("/ch/{}/pan", channel))],
        )
        .await;

    log::info!("Sent subscribe message");

    let mut sync = sync::Sync::new();

    sync.update(sync::Left, 0.0);
    sync.update(sync::Right, 0.0);
    
}
