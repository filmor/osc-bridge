use rosc::{decoder::decode, encoder::encode, OscMessage, OscPacket};
use std::{
    io,
    net::{SocketAddr, UdpSocket},
    sync::mpsc::{channel, Receiver, Sender},
    thread,
    time::Duration,
};
use thiserror::Error;
use thread::JoinHandle;

const BUF_SIZE: usize = 65535;

pub struct OscDevice {
    _thread: JoinHandle<()>,
    send: Sender<OscMessage>,
    recv: Receiver<OscMessage>,
}

impl OscDevice {
    pub fn new(
        name: &str,
        send_addr: impl Into<SocketAddr>,
        recv_addr: impl Into<SocketAddr>,
    ) -> Result<Self, OscDeviceError> {
        let send_addr = send_addr.into();
        let recv_addr = recv_addr.into();

        let name = name.to_owned();

        let (handle, send, recv) = create_thread(name.clone(), send_addr, recv_addr)?;

        Ok(OscDevice {
            _thread: handle,
            send,
            recv,
        })
    }

    pub fn send(&self, msg: OscMessage) {
        self.send.send(msg).expect("Failed to send msg");
    }

    pub fn flush(&self) -> Vec<OscMessage> {
        self.recv.try_iter().collect()
    }
}

fn create_thread(
    name: String,
    send_addr: SocketAddr,
    recv_addr: SocketAddr,
) -> Result<(JoinHandle<()>, Sender<OscMessage>, Receiver<OscMessage>), OscDeviceError> {
    let sock = UdpSocket::bind(recv_addr)?;
    sock.connect(send_addr)?;
    sock.set_read_timeout(Some(Duration::from_millis(1)))?;
    log::info!("Awaiting messages from {}", send_addr);
    log::info!("Listening on {}", sock.local_addr().unwrap());

    let (tx_send, rx_send) = channel();
    let (tx_recv, rx_recv) = channel();

    let thr = thread::spawn(move || {
        let mut buf = vec![0; BUF_SIZE];

        loop {
            while let Ok(len) = sock.recv(&mut buf) {
                if !handle_receive(&name, &buf[..len], &tx_recv) {
                    break;
                };
            }

            // TODO: Check if rx_send is still valid by doing a single peek
            /* match rx_send.recv_timeout(Duration::from_millis(1)) {
                Ok(_) => {}
                Err(_) => {}
            } */
            for msg in rx_send.try_iter() {
                log::debug!("Sending message {:?}", msg);
                handle_send(&name, &sock, msg);
            }

            std::thread::sleep(Duration::from_millis(1));
        }
    });

    Ok((thr, tx_send, rx_recv))
}

fn handle_receive(name: &str, buf: &[u8], tx: &Sender<OscMessage>) -> bool {
    match decode(buf) {
        Ok(OscPacket::Message(msg)) => {
            if tx.send(msg).is_err() {
                log::info!("Failed to forward message, stopping thread");
                return false;
            }
        }
        Ok(OscPacket::Bundle(bdl)) => log::error!("Received unexpected bundle: {:?}", bdl),
        Err(err) => {
            log::error!("[{}] Failed to decode packet: {:?}", name, err);
        }
    }

    true
}

fn handle_send(name: &str, sock: &UdpSocket, msg: OscMessage) {
    match encode(&OscPacket::Message(msg)) {
        Ok(out) => {
            // TODO: log an error
            sock.send(&out).unwrap();
        }
        Err(err) => {
            log::error!("[{}] Failed to encode packet: {:?}", name, err);
        }
    }
}

#[derive(Error, Debug)]
pub enum OscDeviceError {
    #[error("Socket creation failed")]
    Socket(#[from] io::Error),
}
