use bytes::BytesMut;
use rosc::{decoder::decode, encoder::encode, OscMessage, OscPacket, OscType};
use std::{
    io,
    net::{IpAddr, SocketAddr, ToSocketAddrs, UdpSocket},
    sync::mpsc::{channel, Receiver, Sender, SyncSender},
    thread,
    time::Duration,
};
use thiserror::Error;
use thread::JoinHandle;

const BUF_SIZE: usize = 65535;

pub struct OscDevice {
    send_thread: JoinHandle<()>,
    recv_thread: JoinHandle<()>,
    send: Sender<OscPacket>,
    recv: Receiver<OscPacket>,
}

impl OscDevice {
    pub fn new(
        send_addr: impl Into<SocketAddr>,
        recv_addr: impl Into<SocketAddr>,
    ) -> Result<Self, OscDeviceError> {
        let send_addr = send_addr.into();
        let recv_addr = recv_addr.into();

        log::debug!("Starting receive thread...");
        let (recv_thread, recv) = create_recv_thread(send_addr, recv_addr)?;
        log::debug!("Starting send thread...");
        let (send_thread, send) = create_send_thread(send_addr, recv_addr)?;

        Ok(OscDevice {
            send_thread,
            recv_thread,
            send,
            recv,
        })
    }
}

fn create_send_thread(
    send_addr: SocketAddr,
    mut recv_addr: SocketAddr,
) -> Result<(JoinHandle<()>, Sender<OscPacket>), OscDeviceError> {
    recv_addr.set_port(0);
    let sock = UdpSocket::bind(recv_addr)?;
    sock.connect(send_addr)?;

    let (tx, rx) = channel();

    let thr = thread::spawn(move || loop {
        if let Ok(ref msg) = rx.recv() {
            match encode(msg) {
                Ok(out) => {
                    // TODO: log an error
                    sock.send(&out).unwrap();
                }
                Err(err) => {
                    log::error!("Failed to encode packet: {:?}", err);
                }
            }
        } else {
            break;
        }
    });

    Ok((thr, tx))
}

fn create_recv_thread(
    send_addr: SocketAddr,
    recv_addr: SocketAddr,
) -> Result<(JoinHandle<()>, Receiver<OscPacket>), OscDeviceError> {
    let sock = UdpSocket::bind(recv_addr)?;
    sock.connect(send_addr)?;

    let (tx, rx) = channel();

    let thr = thread::spawn(move || {
        let mut buf = vec![0; BUF_SIZE];

        loop {
            match sock.recv(&mut buf) {
                Ok(len) => match decode(&buf[..len]) {
                    Ok(pkt) => {
                        if let Err(_) = tx.send(pkt) {
                            break;
                        }
                    }
                    Err(err) => {
                        log::error!("Failed to decode packet: {:?}", err);
                    }
                },
                Err(err) => {
                    log::error!("Failed to receive from socket: {:?}", err);
                }
            }
        }
    });

    Ok((thr, rx))
}

#[derive(Error, Debug)]
pub enum OscDeviceError {
    #[error("Socket creation failed")]
    Socket(#[from] io::Error),
}
