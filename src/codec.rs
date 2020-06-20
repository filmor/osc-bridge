use bytes::BytesMut;
use rosc::{decoder::decode, encoder::encode, OscPacket};
use std::io;
use tokio_util::codec::{Decoder, Encoder};

pub struct OscCodec;

impl OscCodec {
    pub fn new() -> Self {
        OscCodec {}
    }
}

impl Encoder<OscPacket> for OscCodec {
    type Error = io::Error;

    fn encode(&mut self, msg: OscPacket, buf: &mut BytesMut) -> Result<(), Self::Error> {
        match encode(&msg) {
            Ok(out) => {
                buf.extend_from_slice(&out);
                Ok(())
            }
            Err(err) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("{:?}", err),
            )),
        }
    }
}

impl Decoder for OscCodec {
    type Item = OscPacket;
    type Error = std::io::Error;

    fn decode(&mut self, buf: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        Ok(decode(buf).ok())
    }
}
