mod osc_device;
mod sync;

use get_if_addrs::{get_if_addrs, IfAddr, Interface};
use ipnetwork::Ipv4Network;
use log;
use osc_device::OscDevice;
use pretty_env_logger;
use regex::{Regex, RegexSet};

use rosc::OscMessage;
use std::{collections::HashMap, net::IpAddr, time::Duration};

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
    let wing = OscDevice::new((wing_ip, 2223), (wing_local, 0))
        .expect("Failed to create UDP socket for WING");

    let ds100_regex_set = RegexSet::new(&[
        r"^/dbaudio1/coordinatemapping/source_position_xy/1/",
        r"^/dbaudio1/matrixinput/reverbsendgain/",
        r"^/dbaudio1/reverbinputprocessing/gain/",
    ])
    .unwrap();

    let wing_channel_regex = Regex::new(r"^/ch/(\d+)/send/1/(pan|wid|lvl)$").unwrap();

    let wing_bus_regex = Regex::new(r"^/bus/(\d+)/fdr$").unwrap();

    loop {
        std::thread::sleep(Duration::from_millis(100));

        for msg in ds100.flush() {
            log::info!("Got DS100 message {:?}", msg);
            let ds100_matches = ds100_regex_set.matches(&msg.addr);

            if ds100_matches.matched_any() {
                // map ds100 input
            }
        }

        for msg in wing.flush() {
            log::info!("Got WING message {:?}", msg);
            if let Some(cap) = wing_channel_regex.captures(&msg.addr) {
                // map channel input

                continue;
            }

            if let Some(cap) = wing_bus_regex.captures(&msg.addr) {
                // map channel input

                continue;
            }
        }

        // Send "subscribe"
        // Process "answers"
        // Send new settings
        subscribe_wing(&wing);
        subscribe_ds100(&ds100);
    }

    std::thread::sleep(Duration::from_millis(1000));

    for m in wing.flush() {
        log::info!("{:?}", m);
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

fn empty_msg(addr: String) -> OscMessage {
    OscMessage {
        addr,
        args: Vec::new(),
    }
}

fn subscribe_ds100(device: &OscDevice) {
    for i in 1..=40 {
        let addr = format!("/dbaudio1/matrixinput/reverbsendgain/{}", i);
        device.send(empty_msg(addr));
        let addr = format!("/dbaudio1/coordinatemapping/source_position_xy/1/{}", i);
        device.send(empty_msg(addr));
    }

    for i in 1..=4 {
        let addr = format!("/dbaudio1/reverbinputprocessing/gain/{}", i);
        device.send(empty_msg(addr));
    }
}

fn subscribe_wing(device: &OscDevice) {
    for i in 1..=40 {
        for suffix in &["pan", "wid", "lvl"] {
            let addr = format!("/ch/{}/send/1/{}", i, suffix);
            device.send(empty_msg(addr));
        }
    }

    for i in 1..=4 {
        let addr = format!("/bus/{}/fdr", i);
        device.send(empty_msg(addr));
    }
}
