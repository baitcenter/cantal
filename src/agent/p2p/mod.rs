use std::io::{Read, Write};
use std::net::Ipv4Addr;
use std::sync::{Arc, RwLock, mpsc};
use std::default::Default;
use std::collections::HashMap;

use mio;
use mio::{EventLoop, Token, NonBlock, ReadHint, Handler};
use mio::buf::ByteBuf;
use mio::{Sender, udp};
use cbor::{Decoder, Encoder};
use time::Timespec;
use rustc_serialize::Decodable;

use super::stats::Stats;
use self::peer::{Peer};

mod peer;


const GOSSIP: Token = Token(0);


pub fn p2p_loop(stats: &RwLock<Stats>, host: &str, port: u16,
    sender: mpsc::Sender<mio::Sender<Command>>) {
    let server = udp::bind(&format!("{}:{}", host, port).parse().unwrap()
                            ).unwrap();
    let mut eloop = EventLoop::new().unwrap();
    eloop.register(&server, GOSSIP).unwrap();
    sender.send(eloop.channel()).unwrap();
    let mut ctx = Context {
        sock: server,
        stats: stats.read().unwrap().gossip.clone(),
    };
    eloop.run(&mut ctx).unwrap();
}

#[derive(Debug, Clone, RustcEncodable, RustcDecodable)]
enum Packet {
    Ping {
        myself: Peer,
        now: Timespec,
        friends: Vec<Peer>,
    },
    Pong {
        myself: Peer,
        ping_time: Timespec,
        peer_time: Timespec,
        friends: Vec<Peer>,
    },
}

#[derive(Debug)]
pub enum Command {
    AddGossipHost(Ipv4Addr),
}

struct Context {
    sock: NonBlock<udp::UdpSocket>,
    stats: Arc<RwLock<GossipStats>>,
}

#[derive(Default)]
pub struct GossipStats {
    pub peers: HashMap<Ipv4Addr, Peer>,
}


impl Handler for Context {
    type Timeout = ();
    type Message = Command;

    fn readable(&mut self, eloop: &mut EventLoop<Context>,
                tok: Token, _hint: ReadHint)
    {
        match tok {
            GOSSIP => {
                let mut buf = ByteBuf::mut_with_capacity(4096);
                if let Ok(Some(addr)) = self.sock.recv_from(&mut buf) {
                    let mut dec = Decoder::from_reader(buf.flip());
                    match dec.decode::<Packet>().next() {
                        Some(Ok(packet)) => {
                            println!("Packet {:?} from {:?}", packet, addr);
                        }
                        None => {
                            debug!("Empty packet from {:?}", addr);
                        }
                        Some(Err(e)) => {
                            debug!("Errorneous packet from {:?}: {}", addr, e);
                        }
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    fn notify(&mut self, eloop: &mut EventLoop<Context>, msg: Command) {
        use self::Command::*;
        println!("Command {:?}", msg);
        match msg {
            AddGossipHost(ip) => {
                let ref mut peers = &mut self.stats.write().unwrap().peers;
                if !peers.contains_key(&ip) {
                    peers.insert(ip, Peer {
                        addr: format!("{}", ip),
                        .. Default::default()
                    });
                }
            }
        }
    }
}
