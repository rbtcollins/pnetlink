use ::socket::{NetlinkSocket,NetlinkProtocol};
use libc;

include!(concat!(env!("OUT_DIR"), "/netlink.rs"));

struct NetlinkPacketIterator<'a> {
    sock: &'a NetlinkSocket,
    buf: Vec<u8>,
    idx: usize,
}

impl<'a> Iterator for NetlinkPacketIterator<'a> {
    type Item = NetlinkPacket<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        'done: loop {
            let mut rcvbuf = [0; 4096];
            if let Ok(len) = self.sock.recv(&mut rcvbuf) {
                if len == 0 {
                    break;
                }
                self.buf.extend_from_slice(&rcvbuf[0..len]);
                let nl_payload = &self.buf[self.idx..self.idx+len];
                if let Some(pkt) = NetlinkPacket::new(nl_payload) {
                    let pid = unsafe { libc::getpid() } as u32;
                    let kind = pkt.get_kind();
                    match kind {
                        NLMSG_NOOP => { println!("noop") },
                        NLMSG_ERROR => { println!("err") },
                        NLMSG_DONE => { println!("done"); break 'done; },
                        NLMSG_OVERRUN => { println!("overrun") },
                        _ => {},
                    }
                    return Some(pkt);
                } else {
                    return None;
                }
                self.idx += len;
            }
        }
        None
    }
}
