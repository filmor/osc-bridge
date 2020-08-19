mod osc_device;
mod sync;

use get_if_addrs::{get_if_addrs, IfAddr, Interface};
use ipnetwork::Ipv4Network;
use osc_device::OscDevice;
use regex::{Regex, RegexSet};
use sync::{Side, Sync};

use rosc::{OscMessage, OscType};
use std::{
    net::{IpAddr, Ipv4Addr},
    time::Duration,
};
use structopt::StructOpt;

const MAIN_DELTA: Duration = Duration::from_millis(100);

#[derive(StructOpt)]
struct Cli {
    #[structopt(long)]
    wing_ip: Ipv4Addr,
    #[structopt(long)]
    ds100_ip: Ipv4Addr,
    #[structopt(long)]
    monitor: Vec<i32>,
}

fn main() {
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init_timed();

    let args = Cli::from_args();

    let if_addrs = get_if_addrs().expect("Failed to list local network devices");

    let ds100_ip = IpAddr::V4(args.ds100_ip);
    let wing_ip = IpAddr::V4(args.wing_ip);

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

    let mut x_positions = Vec::new();
    let mut y_positions = Vec::new();
    let mut gains = Vec::new();

    for i in 1..=40 {
        x_positions.push(Sync::new(format!("x{:02}", i)));
        y_positions.push(Sync::new(format!("y{:02}", i)));
        gains.push(Sync::with_transform(
            format!("g{:02}", i),
            gain_ds100_to_wing,
            gain_wing_to_ds100,
        ));
    }

    let mut reverb_gains = Vec::new();

    for i in 1..=4 {
        reverb_gains.push(Sync::with_transform(
            format!("rg{}", i),
            gain_ds100_to_wing,
            gain_wing_to_ds100,
        ));
    }

    subscribe_wing(&wing);
    subscribe_ds100(&ds100);

    loop {
        std::thread::sleep(MAIN_DELTA);

        for msg in ds100.flush() {
            log::debug!("Got DS100 message {:?}", msg);
            let ds100_matches = ds100_regex_set.matches(&msg.addr);

            if ds100_matches.matched_any() {
                let n: usize = msg.addr.rsplit('/').next().unwrap().parse().unwrap();

                match ds100_matches.iter().next().unwrap() {
                    0 => {
                        if let OscType::Float(x) = msg.args[0] {
                            x_positions[n - 1].update(Side::Left, x);
                        }
                        if let OscType::Float(y) = msg.args[1] {
                            y_positions[n - 1].update(Side::Left, y);
                        }
                    }

                    1 => {
                        if let OscType::Float(gain) = msg.args[0] {
                            gains[n - 1].update(Side::Left, gain);
                        }
                    }

                    2 => {
                        if let OscType::Float(revgain) = msg.args[0] {
                            reverb_gains[n - 1].update(Side::Left, revgain);
                        }
                    }

                    _ => {}
                }
            }
        }

        for msg in wing.flush() {
            log::debug!("Got WING message {:?}", msg);
            if let Some(cap) = wing_channel_regex.captures(&msg.addr) {
                // map channel input

                let n: usize = cap[1].parse().unwrap();

                match &cap[2] {
                    "pan" => {
                        if let OscType::Float(x) = msg.args[2] {
                            x_positions[n - 1].update(Side::Right, x);
                        }
                    }
                    "wid" => {
                        if let OscType::Float(x) = msg.args[2] {
                            y_positions[n - 1].update(Side::Right, x);
                        }
                    }
                    "lvl" => {
                        if let OscType::Float(x) = msg.args[2] {
                            gains[n - 1].update(Side::Right, x);
                        }
                    }
                    _ => {}
                }

                continue;
            }

            if let Some(cap) = wing_bus_regex.captures(&msg.addr) {
                // map channel input
                let n: usize = cap[1].parse().unwrap();
                if let OscType::Float(x) = msg.args[2] {
                    reverb_gains[n - 1].update(Side::Right, x);
                }

                continue;
            }
        }

        for i in args.monitor.iter() {
            let n: usize = *i as usize - 1;
            let ref x_sync = x_positions[n];
            let ref y_sync = y_positions[n];
            let ref gain = gains[n];
            log::info!(
                "Channel {}:\tDS100 ({}, {}) @ {}\tWING ({}, {}) @ {}\tMaster: {:?}, {:?}, {:?}",
                n,
                x_sync.left_value() as i32,
                y_sync.left_value() as i32,
                gain.left_value() as i32,
                x_sync.right_value() as i32,
                y_sync.right_value() as i32,
                gain.right_value() as i32,
                x_sync.current_master(),
                y_sync.current_master(),
                gain.current_master()
            );
        }

        for i in 0..40 {
            let n = i + 1;

            match x_positions[i].flush() {
                Some((value, Side::Left)) => {
                    let addr = format!("/dbaudio1/coordinatemapping/source_position_x/1/{}", n);
                    let args = vec![OscType::Float(value)];
                    log::info!(
                        "Sending {:?}",
                        OscMessage {
                            addr: addr.clone(),
                            args: args.clone()
                        }
                    );
                    ds100.send(OscMessage { addr, args });
                }
                Some((value, Side::Right)) => {
                    let addr = format!("/ch/{}/send/1/pan", n);
                    let args = vec![OscType::Float(value)];
                    log::info!(
                        "Sending {:?}",
                        OscMessage {
                            addr: addr.clone(),
                            args: args.clone()
                        }
                    );
                    wing.send(OscMessage { addr, args });
                }
                _ => {}
            }

            // if i == 0 { break; }

            match y_positions[i].flush() {
                Some((value, Side::Left)) => {
                    let addr = format!("/dbaudio1/coordinatemapping/source_position_y/1/{}", n);
                    let args = vec![OscType::Float(value)];
                    ds100.send(OscMessage { addr, args });
                }
                Some((value, Side::Right)) => {
                    let addr = format!("/ch/{}/send/1/wid", n);
                    let args = vec![OscType::Float(value)];
                    wing.send(OscMessage { addr, args });
                }
                _ => {}
            }

            match gains[i].flush() {
                Some((value, Side::Left)) => {
                    let addr = format!("/dbaudio1/matrixinput/reverbsendgain/{}", n);
                    let args = vec![OscType::Float(value)];
                    ds100.send(OscMessage { addr, args });
                }
                Some((value, Side::Right)) => {
                    let addr = format!("/ch/{}/send/1/lvl", n);
                    let args = vec![OscType::Float(value)];
                    wing.send(OscMessage { addr, args });
                }
                _ => {}
            }
        }

        for i in 0..4 {
            let n = i + 1;
            match reverb_gains[i].flush() {
                Some((value, Side::Left)) => {
                    let addr = format!("/dbaudio1/reverbinputprocessing/gain/{}", n);
                    let args = vec![OscType::Float(value)];
                    ds100.send(OscMessage { addr, args });
                }
                Some((value, Side::Right)) => {
                    let addr = format!("/bus/{}/fdr", n);
                    let args = vec![OscType::Float(value)];
                    wing.send(OscMessage { addr, args });
                }
                _ => {}
            }
        }

        // Send new settings
        subscribe_wing(&wing);
        subscribe_ds100(&ds100);
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
        IpAddr::V6(_addr) => {
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

fn gain_wing_to_ds100(val: f32) -> f32 {
    if val > 0.0 {
        val / 10.0 * 24.0
    } else {
        val / 144.0 * 120.0
    }
}

fn gain_ds100_to_wing(val: f32) -> f32 {
    if val > 0.0 {
        val / 24.0 * 10.0
    } else {
        val / 120.0 * 140.0
    }
}
