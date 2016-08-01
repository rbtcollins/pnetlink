use ::socket::{NetlinkSocket,NetlinkProtocol};
use libc;
use std::mem;

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

fn align(len: usize) -> usize {
    const RTA_ALIGNTO: usize = 4;

    ((len)+RTA_ALIGNTO-1) & !(RTA_ALIGNTO-1)
}

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
