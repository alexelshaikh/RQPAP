use std::net::{TcpStream, SocketAddr, IpAddr, Ipv4Addr};
use std::io::{Read, Write, Error};
use std::str;
use std::sync::Arc;
use std::time::Duration;
use crate::base_sequence::BaseSequence;
use crate::safe_cell::SafeCell;
use parking_lot::{Mutex, RawMutex};
use parking_lot::lock_api::MutexGuard;

pub struct DGClient {
    channels: Vec<ChannelHandler>,
}
/// The client used to communicate with the dg server.
impl DGClient {
    /// Creates a new DGClient instance.
    /// # Arguments
    /// * The arguments `a`, `b`, `c`, and `d` represent the IP address of the dg server. For example, if the IP is 127.0.0.1, then `a` = 127, `b` = 0, `c` = 0, and `d` = 1.
    /// * `start_port` - The starting port of the dg server.
    /// * `count` - The number of ports (including `start_port`).
    pub fn new(a: u8, b: u8, c: u8, d: u8, start_port: u16, count: u16) -> Option<DGClient> {
        let channels = (start_port..start_port + count)
            .map(|port| ChannelHandler::new(a, b, c, d, port))
            .take_while(|c| c.is_some())
            .map(|c| c.unwrap())
            .collect::<Vec<_>>();
        if channels.len() == count as usize {
            Some(DGClient {
                channels
            })
        }
        else {
            None
        }
    }

    /// Returns the dg energy for a given `seq`. Will loop over all ports (channels) to send the query. Will start at port `from_id`.
    #[inline(always)]
    pub fn dg_arc_from_id(&self, mut from_id: usize, seq: &Arc<BaseSequence>, temp: f32) -> f32 {
        let mut safe_id = from_id % self.channels.len();
        loop {
            match self.channels.get(safe_id).unwrap().stream.try_lock() {
                None => {
                    safe_id = (safe_id + 1) % self.channels.len();
                }
                Some(ch) => {
                    return ChannelHandler::send_seq_receive_dg_arc_lock_free(ch,seq, temp);
                }
            };
        }
    }

    /// Returns the dg energy for a given `seq`. Will loop over all ports (channels) to send the query. Will start at port `start_port`.
    #[inline(always)]
    pub fn dg_arc(&self, seq: &Arc<BaseSequence>, temp: f32) -> f32 {
        self.dg_arc_from_id(0_usize, seq, temp)
    }
}

pub struct ChannelHandler {
    stream: Mutex<TcpStream>
}

impl ChannelHandler {
    /// Creates a single channel (port) through which queries can be sent.
    /// # Arguments
    /// * The arguments `a`, `b`, `c`, and `d` represent the IP address. For example, if the IP is 127.0.0.1, then `a` = 127, `b` = 0, `c` = 0, and `d` = 1.
    /// * `port` - The port of this channel.
    pub fn new(a: u8, b: u8, c: u8, d: u8, port: u16) -> Option<ChannelHandler> {
        let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(a, b, c, d)), port);
        match TcpStream::connect_timeout(&socket, Duration::from_secs(3)) {
            Ok(st) => Some(
                ChannelHandler {
                    stream: Mutex::new(st)
            }),
            Err(_) => None
        }
    }

    fn send_seq_receive_dg(&mut self, seq: &BaseSequence, temp: f32) -> f32 {
        let mut locked = self.stream.lock();
        let mut packet_data: Vec<u8> = Vec::with_capacity(seq.len() + 4 + 1);
        packet_data.extend_from_slice(seq.to_string().as_bytes());
        packet_data.push(b',');
        packet_data.extend_from_slice((temp.to_string()).as_ref());
        locked.write_all(packet_data.as_slice());
        locked.flush().unwrap();
        let mut buffer = [0u8; 4];
        match locked.read_exact(&mut buffer) {
            Ok(_) => {
                f32::from_le_bytes(buffer)
            }
            Err(_) => {
                0_f32
            }
        }
    }


    fn send_seq_receive_dg_arc(&mut self, seq: Arc<BaseSequence>, temp: f32) -> f32 {
        let mut locked = self.stream.lock();
        let mut packet_data: Vec<u8> = Vec::with_capacity(seq.len() + 4 + 1);
        packet_data.extend_from_slice(seq.to_string().as_bytes());
        packet_data.push(b',');
        packet_data.extend_from_slice((temp.to_string()).as_ref());
        locked.write_all(packet_data.as_slice());
        locked.flush().unwrap();
        let mut buffer = [0u8; 4];
        match locked.read_exact(&mut buffer) {
            Ok(_) => {
                f32::from_le_bytes(buffer)
            }
            Err(_) => {
                0_f32
            }
        }
    }

    #[inline]
    fn send_seq_receive_dg_arc_lock_free(mut locked: MutexGuard<RawMutex, TcpStream>, seq: &Arc<BaseSequence>, temp: f32) -> f32 {
        let mut packet_data: Vec<u8> = Vec::with_capacity(seq.len() + 4 + 1);
        packet_data.extend_from_slice(seq.to_string().as_bytes());
        packet_data.push(b',');
        packet_data.extend_from_slice((temp.to_string()).as_ref());
        locked.write_all(packet_data.as_slice());
        locked.flush().unwrap();
        let mut buffer = [0u8; 4];
        match locked.read_exact(&mut buffer) {
            Ok(_) => {
                f32::from_le_bytes(buffer)
            }
            Err(_) => {
                0_f32
            }
        }
    }
}