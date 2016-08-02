use packet::netlink::{MutableNetlinkPacket,NetlinkPacket};
use packet::netlink::{NLM_F_REQUEST, NLM_F_DUMP};
use packet::netlink::{NLMSG_NOOP,NLMSG_ERROR,NLMSG_DONE,NLMSG_OVERRUN};
use ::socket::{NetlinkSocket,NetlinkProtocol};
use packet::netlink::{NetlinkPacketIterator,NetlinkConnection,NetlinkConnectionIterator};
use pnet::packet::MutablePacket;
use pnet::packet::Packet;
use pnet::packet::PacketSize;
use libc;

include!(concat!(env!("OUT_DIR"), "/route.rs"));

const RTA_ALIGNTO: usize = 4;

/* rt message types */
pub const RTM_NEWLINK: u16 = 16;
pub const RTM_DELLINK: u16 = 17;
pub const RTM_GETLINK: u16 = 18;
pub const RTM_SETLINK: u16 = 19;

pub const RTM_NEWADDR: u16 = 20;

/* attributes */
pub const IFLA_UNSPEC: u16 = 0;
pub const IFLA_ADDRESS: u16 = 1;
pub const IFLA_BROADCAST: u16 = 2;
pub const IFLA_IFNAME: u16 = 3;
pub const IFLA_MTU: u16 = 4;
pub const IFLA_LINK: u16 = 5;
pub const IFLA_QDISC: u16 = 6;
pub const IFLA_STATS: u16 = 7;
pub const IFLA_COST: u16 = 8;
pub const IFLA_PRIORITY: u16 = 9;
pub const IFLA_MASTER: u16 = 10;
pub const IFLA_WIRELESS: u16 = 11;
pub const IFLA_PROTINFO: u16 = 12;
pub const IFLA_TXQLEN: u16 = 13;
pub const IFLA_MAP: u16 = 14;
pub const IFLA_WEIGHT: u16 = 15;
pub const IFLA_OPERSTATE: u16 = 16;
pub const IFLA_LINKMODE: u16 = 17;
pub const IFLA_LINKINFO: u16 = 18;
pub const IFLA_NET_NS_PID: u16 = 19;
pub const IFLA_IFALIAS: u16 = 20;
pub const IFLA_NUM_VF: u16 = 21;
pub const IFLA_VFINFO_LIST: u16 = 22;
pub const IFLA_STATS64: u16 = 23;
pub const IFLA_VF_PORTS: u16 = 24;
pub const IFLA_PORT_SELF: u16 = 25;
pub const IFLA_AF_SPEC: u16 = 26;

pub const IFLA_INFO_UNSPEC: u16 = 0;
pub const IFLA_INFO_KIND: u16 = 1;
pub const IFLA_INFO_DATA: u16 = 2;
pub const IFLA_INFO_XSTATS: u16 = 3;

fn align(len: usize) -> usize {
    ((len)+RTA_ALIGNTO-1) & !(RTA_ALIGNTO-1)
}

struct Link<'a> {
    packet: IfInfoPacket<'a>,
}

impl<'a> Link<'a> {
    fn dump_links_request<'b>(buf: &'b mut [u8]) -> NetlinkPacket<'b> {
        let len = MutableNetlinkPacket::minimum_packet_size() + MutableIfInfoPacket::minimum_packet_size();
        {
            let mut pkt = MutableNetlinkPacket::new(buf).unwrap();
            {
                pkt.set_length(len as u32);
                pkt.set_flags(NLM_F_REQUEST | NLM_F_DUMP);
                pkt.set_kind(RTM_GETLINK);
                let mut ifinfo_buf = pkt.payload_mut();
                let mut ifinfo = MutableIfInfoPacket::new(&mut ifinfo_buf).unwrap();
                ifinfo.set_family(0 /* AF_UNSPEC */);
            }
        }
        NetlinkPacket::new(buf).unwrap()
    }

    fn dump_links() {
        let mut conn = NetlinkConnection::new();
        let mut buf = [0; 32];
        let mut reply = conn.send(Self::dump_links_request(&mut buf));

        while let Ok(Some(pkt)) = reply.read_netlink() {
            println!("{:?}", pkt);
            /*
            if pkt.kind == NLMSG_DONE {
                break;
            }
            */
        }

        /*
        for pkt in reply {
            println!("PKT: {:?}", pkt);
        }
        */
    }

    /*
    fn dump_links() {
        use std::ffi::CStr;
        use packet::netlink::NetlinkPacketIterator;
        let mut sock = NetlinkSocket::bind(NetlinkProtocol::Route, 0 as u32).unwrap();
        sock.send(&Link::dump_links_request()).unwrap();

        let iter = NetlinkPacketIterator::new(&mut sock);
        for msg in iter {
            println!("{:?}", msg);
            if msg.get_kind() != RTM_NEWLINK {
                println!("bad type!");
                continue;
            }

            if let Some(ifi) = IfInfoPacket::new(&msg.payload()[0..]) {
                println!("├ ifi: {:?}", ifi);
                let payload = &ifi.payload()[0..];
                let iter = RtAttrIterator::new(payload);
                for rta in iter {
                    match rta.get_rta_type() {
                        IFLA_IFNAME => {
                            println!(" ├ ifname: {:?}", CStr::from_bytes_with_nul(rta.payload()));
                        },
                        IFLA_ADDRESS => {
                            println!(" ├ hw addr: {:?}", rta.payload());
                        },
                        IFLA_LINKINFO => {
                            println!(" ├ LINKINFO {:?}", rta);
                        },
                        _ => {
                            println!(" ├ {:?}", rta);
                        },
                    }
                }
            }
        }
    }
    */
}

pub struct RtAttrIterator<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> RtAttrIterator<'a> {
    fn new(buf: &'a [u8]) -> Self {
        RtAttrIterator {
            buf: buf,
            pos: 0,
        }
    }
}

impl<'a> Iterator for RtAttrIterator<'a> {
    type Item = RtAttrPacket<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(rta) = RtAttrPacket::new(&self.buf[self.pos..]) {
            let len = rta.get_rta_len() as usize; 
            if len < 4 {
                return None;
            }
            self.pos += align(len as usize);
            return Some(rta);
        }
        None
    }
}

#[test]
fn netlink_route_dump_links() {
    Link::dump_links();
}
