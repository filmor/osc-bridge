use rosc::{OscMessage, OscPacket, OscType};
use std::{net::{UdpSocket, SocketAddr, ToSocketAddrs}, pin::Pin};
use std::{thread, sync::mpsc::{channel}};

pub struct OscDevice {
    thread_handle: thread::JoinHandle<()>,
}

impl OscDevice {
    pub fn new(send: impl ToSocketAddrs, recv: impl ToSocketAddrs) -> Self {
        let sock = UdpSocket::bind(recv);

        let thread_handle = thread::spawn(|| {

        });

        OscDevice { thread_handle }
    }
}