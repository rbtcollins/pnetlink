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

/* XXX: NetlinkIterable */
pub struct NetlinkPacketIterator<'a> {
    buf: &'a [u8],
}

impl<'a> NetlinkPacketIterator<'a> {
    fn new(buf: &'a [u8]) -> Self {
        NetlinkPacketIterator {
            buf: buf 
        }
    }
}

impl<'a> Iterator for NetlinkPacketIterator<'a> {
    type Item = NetlinkPacket<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(pkt) = NetlinkPacket::new(self.buf) {
            let len = align(pkt.get_length() as usize);
            self.buf = &self.buf[len..];
            return Some(pkt);
        }
        None
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

    let it = NetlinkPacketIterator::new(&data);
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
        println!("{:?}", pkt);
        if pkt.kind == NLMSG_DONE {
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
        println!("{:?}", pkt);
    }
}

pub struct NetlinkReader<R: Read> {
    reader: R,
    buf: Vec<u8>,
    write_at: usize,
    read_at: usize,
}

impl<R: Read> NetlinkReader<R> {
    fn new(reader: R) -> Self {
        NetlinkReader {
            reader: reader,
            buf: vec![],
            write_at: 0,
            read_at: 0,
        }
    }
}

impl<R: Read> NetlinkReader<R> {
    pub fn read_netlink(&mut self) -> io::Result<Option<Netlink>> {
        loop {
            loop {
                if let Some(pkt) = NetlinkPacket::new(&self.buf[self.read_at..]) {
                    let len = align(pkt.get_length() as usize);
                    if len == 0 {
                        return Ok(None);
                    }
                    self.read_at += len;
                    return Ok(Some(pkt.from_packet()));
                }
                break;
            }
            let mut buf = [0; 4096];
            match self.reader.read(&mut buf) {
                Ok(_) => self.buf.extend_from_slice(&buf),
                Err(e) => return Err(e),
            }
        }
    }
}

pub struct NetlinkConnection {
    sock: NetlinkSocket,
    buf: Vec<u8>,
}

impl NetlinkConnection {
    pub fn new() -> Self {
        NetlinkConnection {
            sock: NetlinkSocket::bind(NetlinkProtocol::Route, 0 as u32).unwrap(),
            buf: vec![],
        }
    }

    pub fn send<'a,'b>(&'a mut self, msg: NetlinkPacket<'b>) -> NetlinkReader<&'a mut NetlinkSocket> {
        self.sock.send(msg.packet()).unwrap();
        NetlinkReader::new(&mut self.sock)
        /*
        NetlinkConnectionIterator {
            sock: &mut self.sock,
            buf: &mut self.buf,
            pos: 0,
        }
        */
    }
}

pub struct NetlinkConnectionIterator<'a> {
    sock: &'a mut NetlinkSocket,
    buf: &'a mut Vec<u8>,
    pos: usize,
}

impl<'a> Iterator for NetlinkConnectionIterator<'a> {
    type Item = NetlinkPacket<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

/*
pub struct NetlinkPacketIterator<'a> {
    sock: &'a mut NetlinkSocket,
    buf: Vec<u8>,
    read_at: usize, // read index
}

impl<'a> NetlinkPacketIterator<'a> {
    pub fn new(sock: &'a mut NetlinkSocket) -> Self {
        NetlinkPacketIterator {
            sock: sock,
            buf: vec![],
            read_at: 0,
        }
    }
}

pub struct NetlinkConnection<'a> {
    sock: &'a mut NetlinkSocket,
    buf: Vec<u8>
}
*/

/*
impl<'a> Iterator for NetlinkPacketIterator<'a> {
    type Item = NetlinkPacket<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            loop {
                // TODO: figure out how to make this safe
                let nl_payload = unsafe { mem::transmute(&self.buf[self.read_at..]) };
                if let Some(pkt) = NetlinkPacket::new(nl_payload) {
                    let pid = unsafe { libc::getpid() } as u32;
                    let kind = pkt.get_kind();
                    match kind {
                        NLMSG_NOOP => { println!("noop") },
                        NLMSG_ERROR => {
                            println!("err");
                            return None;
                        },
                        NLMSG_DONE => {
                            println!("done");
                            return None;
                        },
                        NLMSG_OVERRUN => { println!("overrun") },
                        _ => {
                            println!("KIND IS {}", kind);
                        },
                    }

                    if pkt.get_pid() != pid {
                        println!("wrong pid!");
                        continue;
                    }

                    self.read_at += align(pkt.get_length() as usize);
                    if self.read_at == 0 {
                        break;
                    }
                    return Some(pkt);
                } else {
                    break;
                }
            }
            let mut rcvbuf = [0; 4096];
            let sock = &mut self.sock;
            if let Ok(len) = sock.recv(&mut rcvbuf) {
                if len == 0 {
                    break;
                }
                self.buf.extend_from_slice(&rcvbuf[0..len]);
            } else {
                break;
            }
        }
        None
    }
}
*/
