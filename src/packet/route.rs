use packet::netlink::{MutableNetlinkPacket,NetlinkPacket};
use packet::netlink::{NLM_F_REQUEST, NLM_F_DUMP};
use packet::netlink::{NLMSG_NOOP,NLMSG_ERROR,NLMSG_DONE,NLMSG_OVERRUN};
use ::socket::{NetlinkSocket,NetlinkProtocol};
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

struct Link {
}

impl Link {
    fn dump_links_request<'a>() -> [u8;32] {
        let len = MutableNetlinkPacket::minimum_packet_size() + MutableIfInfoPacket::minimum_packet_size();
        let mut buf = [0; 32];
        assert!(len == 32);
        {
            let mut pkt = MutableNetlinkPacket::new(&mut buf).unwrap();
            pkt.set_length(len as u32);
            pkt.set_flags(NLM_F_REQUEST | NLM_F_DUMP);
            pkt.set_kind(RTM_GETLINK);
            let mut ifinfo_buf = pkt.payload_mut();
            let mut ifinfo = MutableIfInfoPacket::new(&mut ifinfo_buf).unwrap();
            ifinfo.set_family(0 /* AF_UNSPEC */);
        }
        buf
    }

    fn read_netlink_response(sock: &mut NetlinkSocket) {
        'done: loop {
            let mut rcvbuf = [0; 4096];
            let mut big_buff = vec![0; 4096];
            if let Ok(len) = sock.recv(&mut rcvbuf) {
                if len == 0 {
                    break;
                }
                let mut nl_payload = &rcvbuf[0..len];
                //println!("{:?}", nl_payload);
                big_buff.extend_from_slice(&nl_payload[..]);
                let mut pkt_idx = 0;
                loop {
                    //println!("PKT IDX: {}", pkt_idx);
                    if let Some(msg) = NetlinkPacket::new(nl_payload) {
                        let pid = unsafe { libc::getpid() } as u32;
                        let kind = msg.get_kind();
                        match kind {
                            NLMSG_NOOP => { println!("noop") },
                            NLMSG_ERROR => { println!("err") },
                            NLMSG_DONE => { println!("done"); break 'done; },
                            NLMSG_OVERRUN => { println!("overrun") },
                            _ => {},
                        }
                        pkt_idx = align(msg.get_length() as usize);
                        if pkt_idx == 0 {
                            break;
                        }
                        nl_payload = &nl_payload[pkt_idx..];

                        println!("{:?} {}", msg, pid);

                        if msg.get_pid() != pid {
                            println!("wrong pid!");
                            continue;
                        }
                        if msg.get_kind() != RTM_NEWLINK {
                            println!("bad type!");
                            continue;
                        }
                    } else {
                        break;
                    }
                }
            }
        }
    }

    fn dump_links() {
        let mut sock = NetlinkSocket::bind(NetlinkProtocol::Route, 0 as u32).unwrap();
        sock.send(&Self::dump_links_request()).unwrap();
        Self::read_netlink_response(&mut sock);
    }
}

#[test]
fn netlink_route_dump_links_2() {
    Link::dump_links();
}

#[test]
fn netlink_route_dump_links_3() {
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
            let mut payload = &ifi.payload()[0..];
            let total_len = payload.len();
            let mut idx = 0;
            loop {
                if let Some(rta) = RtAttrPacket::new(payload) {
                    let len = rta.get_rta_len() as usize;
                    //println!("RTA LEN: {}, TOTAL: {}", len, total_len - idx);
                    if len > total_len - idx || len < 4 {
                        break;
                    }
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
                    let mut align = align(len);
                    idx += align;
                    payload = &payload[align..];
                } else {
                    //println!("CANT PARSE RTATTR");
                    break;
                }
            }
        }
    }
}

#[test]
fn netlink_route_dump_links_1() {
    use libc;
    use packet::netlink::{MutableNetlinkPacket,NetlinkPacket};
    use packet::route::{MutableIfInfoPacket,IfInfoPacket};
    use packet::route::RtAttrPacket;
    use packet::netlink::{NLM_F_REQUEST, NLM_F_DUMP};
    use packet::netlink::{NLMSG_NOOP,NLMSG_ERROR,NLMSG_DONE,NLMSG_OVERRUN};
    use packet::route::{RTM_GETLINK,RTM_NEWLINK,IFLA_IFNAME,IFLA_ADDRESS,IFLA_LINKINFO};
    use socket::{NetlinkSocket,NetlinkProtocol};
    use pnet::packet::MutablePacket;
    use pnet::packet::Packet;
    use pnet::packet::PacketSize;
    use std::ffi::CStr;

    let mut sock = NetlinkSocket::bind(NetlinkProtocol::Route, 0 as u32).unwrap();
    let mut buf = [0; 1024];
    {
        let mut pkt = MutableNetlinkPacket::new(&mut buf).unwrap();
        pkt.set_length(MutableNetlinkPacket::minimum_packet_size() as u32 + 
                MutableIfInfoPacket::minimum_packet_size() as u32);
        pkt.set_flags(NLM_F_REQUEST | NLM_F_DUMP/*| flags */);
        pkt.set_kind(RTM_GETLINK);
        let mut ifinfo_buf = pkt.payload_mut();
        let mut ifinfo = MutableIfInfoPacket::new(&mut ifinfo_buf).unwrap();
        ifinfo.set_family(0 /* AF_UNSPEC */);
    }
    
    sock.send(&buf[0..32]);
    'done: loop {
    let mut rcvbuf = [0; 4096];
    let mut big_buff = vec![0; 4096];
    if let Ok(len) = sock.recv(&mut rcvbuf) {
        if len == 0 {
            break;
        }
        let mut nl_payload = &rcvbuf[0..len];
        //println!("{:?}", nl_payload);
        big_buff.extend_from_slice(&nl_payload[..]);
        let mut pkt_idx = 0;
        loop {
            //println!("PKT IDX: {}", pkt_idx);
            if let Some(msg) = NetlinkPacket::new(nl_payload) {
                let pid = unsafe { libc::getpid() } as u32;
                let kind = msg.get_kind();
                match kind {
                    NLMSG_NOOP => { println!("noop") },
                    NLMSG_ERROR => { println!("err") },
                    NLMSG_DONE => { println!("done"); break 'done; },
                    NLMSG_OVERRUN => { println!("overrun") },
                    _ => {},
                }
                pkt_idx = align(msg.get_length() as usize);
                if pkt_idx == 0 {
                    break;
                }
                nl_payload = &nl_payload[pkt_idx..];

                println!("{:?} {}", msg, pid);

                if msg.get_pid() != pid {
                    println!("wrong pid!");
                    continue;
                }
                if msg.get_kind() != RTM_NEWLINK {
                    println!("bad type!");
                    continue;
                }
            
                if let Some(ifi) = IfInfoPacket::new(&msg.payload()[0..]) {
                    println!("├ ifi: {:?}", ifi);
                    let mut payload = &ifi.payload()[0..];
                    let total_len = payload.len();
                    let mut idx = 0;
                    loop {
                        if let Some(rta) = RtAttrPacket::new(payload) {
                            let len = rta.get_rta_len() as usize;
                            //println!("RTA LEN: {}, TOTAL: {}", len, total_len - idx);
                            if len > total_len - idx || len < 4 {
                                break;
                            }
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
                            let mut align = align(len);
                            idx += align;
                            payload = &payload[align..];
                        } else {
                            //println!("CANT PARSE RTATTR");
                            break;
                        }
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }
    }
}
