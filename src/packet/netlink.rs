use ::socket::{NetlinkSocket,NetlinkProtocol};
use libc;
use std::mem;
use std::io;
use std::io::{Read,BufRead,BufReader};
use std::marker::PhantomData;
use pnet::packet::{Packet,FromPacket};

include!(concat!(env!("OUT_DIR"), "/netlink.rs"));

/* It is request message. 	*/
pub const NLM_F_REQUEST: u16 = 1;
/* Multipart message, terminated by NLMSG_DONE */
pub const NLM_F_MULTI: u16 = 2;
/* Reply with ack, with zero or error code */
pub const NLM_F_ACK: u16 = 4;
/* Echo this request 		*/
pub const NLM_F_ECHO: u16 = 8;
/* Dump was inconsistent due to sequence change */
pub const NLM_F_DUMP_INTR: u16 = 16;

/* Modifiers to GET request */
pub const NLM_F_ROOT: u16 =	0x100;	/* specify tree	root	*/
pub const NLM_F_MATCH: u16 = 0x200;	/* return all matching	*/
pub const NLM_F_ATOMIC: u16 = 0x400;	/* atomic GET		*/
pub const NLM_F_DUMP: u16 =	(NLM_F_ROOT|NLM_F_MATCH);

/* message types */
pub const NLMSG_NOOP: u16 = 1;
pub const NLMSG_ERROR: u16 = 2;
pub const NLMSG_DONE: u16 = 3;
pub const NLMSG_OVERRUN: u16 = 4;

fn align(len: usize) -> usize {
    const RTA_ALIGNTO: usize = 4;

    ((len)+RTA_ALIGNTO-1) & !(RTA_ALIGNTO-1)
}

impl<'a> NetlinkIterable<'a> {
    pub fn new(buf: &'a [u8]) -> Self {
        NetlinkIterable { buf: buf }
    }
}

#[test]
fn read_ip_link_dump() {
    use std::fs::File;
    use std::io::prelude;
    use std::io::BufReader;
    use std::io::BufRead;
    use std::io::Read;

    let f = File::open("dumps/ip_link.bin").unwrap();
    let mut r = BufReader::new(f);
    let mut data = vec![];
    r.read_to_end(&mut data).unwrap();

    let it = NetlinkIterable::new(&data);
    for pkt in it {
        println!("{:?}", pkt);
    }
}

#[test]
fn read_ip_link_dump_2() {
    use std::fs::File;
    use std::io::prelude;
    use std::io::BufReader;
    use std::io::BufRead;
    use std::io::Read;

    let f = File::open("dumps/ip_link.bin").unwrap();
    let mut r = BufReader::new(f);
    let mut reader = NetlinkReader::new(r);
    while let Ok(Some(pkt)) = reader.read_netlink() {
        let pkt = pkt.get_packet();
        println!("{:?}", pkt);
        if pkt.get_kind() == NLMSG_DONE {
            break;
        }
    }
}

#[test]
fn read_ip_link_sock() {
    use std::fs::File;
    use std::io::prelude;
    use std::io::BufReader;
    use std::io::BufRead;
    use std::io::Read;

    let mut r = NetlinkSocket::bind(NetlinkProtocol::Route, 0 as u32).unwrap();
    let mut reader = NetlinkReader::new(r);
    while let Ok(Some(pkt)) = reader.read_netlink() {
        let pkt = pkt.get_packet();
        println!("{:?}", pkt);
    }
}

pub struct NetlinkSlot {
    data: Vec<u8>,
}

impl NetlinkSlot {
    fn new(data: &[u8]) -> Self {
        NetlinkSlot { data: data.to_owned() }
    }

    pub fn get_packet(&self) -> NetlinkPacket {
        NetlinkPacket::new(&self.data[..]).unwrap()
    }
}

pub struct NetlinkReader<R: Read> {
    reader: R,
    buf: Vec<u8>,
    read_at: usize,
    state: NetlinkReaderState,
}

enum NetlinkReaderState {
    Done,
    NeedMore,
    Error,
    Parsing,
}

impl<R: Read> NetlinkReader<R> {
    fn new(reader: R) -> Self {
        NetlinkReader {
            reader: reader,
            buf: vec![],
            read_at: 0,
            state: NetlinkReaderState::NeedMore,
        }
    }
}

impl<R: Read> ::std::iter::IntoIterator for NetlinkReader<R> {
    type Item = NetlinkSlot;
    type IntoIter = NetlinkSlotIterator<R>;

    fn into_iter(self) -> Self::IntoIter {
        NetlinkSlotIterator { reader: self }
    }
}

impl<R: Read> NetlinkReader<R> {
    pub fn read_netlink(&mut self) -> io::Result<Option<NetlinkSlot>> {
        loop {
            match self.state {
                NetlinkReaderState::NeedMore => {
                    let mut buf = [0; 4096];
                    match self.reader.read(&mut buf) {
                        Ok(0) => {
                            self.state = NetlinkReaderState::Done;
                            return Ok(None);
                        },
                        Ok(len) =>{
                            self.buf.extend_from_slice(&buf[0..len]);
                        },
                        Err(e) => {
                            self.state = NetlinkReaderState::Error;
                            return Err(e);
                        }
                    }
                },
                NetlinkReaderState::Done => return Ok(None),
                NetlinkReaderState::Error => return Ok(None),
                NetlinkReaderState::Parsing => { },
            }
            loop {
                if let Some(pkt) = NetlinkPacket::new(&self.buf[self.read_at..]) {
                    let len = align(pkt.get_length() as usize);
                    if len == 0 {
                        return Ok(None);
                    }
                    match pkt.get_kind() {
                        NLMSG_ERROR => {
                            self.state = NetlinkReaderState::Error;
                            return Ok(None); /* XXX: fix me */
                        },
                        NLMSG_OVERRUN => {
                            panic!("overrun!");
                        },
                        NLMSG_DONE => {
                            self.state = NetlinkReaderState::Done;
                        },
                        NLMSG_NOOP => {
                            println!("noop")
                        },
                        _ => {
                            self.state = NetlinkReaderState::Parsing;
                        },
                    }
                    let slot = NetlinkSlot::new(&self.buf[self.read_at..self.read_at + pkt.get_length() as usize]);
                    self.read_at += len;
                    return Ok(Some(slot));
                } else {
                    self.state = NetlinkReaderState::NeedMore;
                    break;
                }
            }
        }
    }
}

pub struct NetlinkSlotIterator<R: Read> {
    reader: NetlinkReader<R>,
}

impl<R: Read> Iterator for NetlinkSlotIterator<R> {
    type Item = NetlinkSlot;

    fn next(&mut self) -> Option<Self::Item> {
        match self.reader.read_netlink() {
            Ok(Some(slot)) => Some(slot),
            _ => None,
        }
    }
}

pub struct NetlinkConnection {
    sock: NetlinkSocket,
}

impl NetlinkConnection {
    pub fn new() -> Self {
        NetlinkConnection {
            sock: NetlinkSocket::bind(NetlinkProtocol::Route, 0 as u32).unwrap(),
        }
    }

    pub fn send<'a,'b>(&'a mut self, msg: NetlinkPacket<'b>) -> NetlinkReader<&'a mut NetlinkSocket> {
        self.sock.send(msg.packet()).unwrap();
        NetlinkReader::new(&mut self.sock)
    }
}
