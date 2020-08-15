mod osc_device;
mod sync;

use get_if_addrs::{get_if_addrs, IfAddr, Interface};
use ipnetwork::Ipv4Network;
use log;
use osc_device::OscDevice;
use pretty_env_logger;

use std::net::IpAddr;

fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init_timed();

    let if_addrs = get_if_addrs().expect("Failed to list local network devices");

    let ds100_ip = IpAddr::V4(
        "192.168.178.75"
            .parse()
            .expect("Failed to parse DS100 address"),
    );
    let wing_ip = IpAddr::V4(
        "192.168.178.41"
            .parse()
            .expect("Failed to parse WING address"),
    );

    let ds100_local = get_matching_interface(ds100_ip, &if_addrs)
        .expect("Failed to find matching local interface");
    let wing_local = get_matching_interface(wing_ip, &if_addrs)
        .expect("Failed to find matching local interface");

    log::info!("Connecting to DS100...");
    let ds100 = OscDevice::new((ds100_ip, 50010), (ds100_local, 50011))
        .expect("Failed to create UDP socket for DS100");
    log::info!("Connecting to WING...");
    let wing = OscDevice::new((wing_local, 2223), (wing_local, 2223))
        .expect("Failed to create UDP socket for WING");
    
    for i in 1..=48 {
        
    }
}

fn get_matching_interface(addr: IpAddr, interfaces: &Vec<Interface>) -> Option<IpAddr> {
    match addr {
        IpAddr::V4(addr) => {
            for interface in interfaces.iter() {
                if let IfAddr::V4(ref if_addr) = interface.addr {
                    if let Ok(net) = Ipv4Network::with_netmask(if_addr.ip, if_addr.netmask) {
                        if net.contains(addr) {
                            log::info!(
                                "Using device '{}' ({}) to connect to {}",
                                interface.name,
                                if_addr.ip,
                                addr
                            );
                            return Some(IpAddr::V4(if_addr.ip));
                        }
                    }
                }
            }
        }
        IpAddr::V6(addr) => {
            unimplemented!("IPv6 is not supported");
        }
    }

    log::error!("No matching local interface found for {}", addr);
    None
}

/*
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
*/
