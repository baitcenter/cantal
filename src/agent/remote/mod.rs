use std::mem::replace;
use std::net::{SocketAddr};
use std::ops::DerefMut;
use std::str::from_utf8;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;

use mio::{Token, Timeout, EventSet, Sender};
use mio::util::Slab;
use time::{SteadyTime, Duration};
use rand::thread_rng;
use rand::distributions::{IndependentSample, Range};
use rustc_serialize::json;

use super::server::Context;
use super::scan::time_ms;
use super::websock::{Beacon, write_text};
use super::websock::InputMessage as OutputMessage;
use super::websock::OutputMessage as InputMessage;
use super::deps::{LockedDeps};
use super::server::Timer::{ReconnectPeer, ResetPeer};
use super::p2p::GossipStats;
use super::p2p;
use self::owebsock::WebSocket;
use super::rules::{RawQuery, RawRule, RawResult};
use super::rules;
use super::history::History;
use super::http;
use super::http::{Request, BadRequest};
use super::ioutil::Poll;
use super::server::{MAX_OUTPUT_BUFFER};
use super::server::Timer::{RemoteCollectGarbage};

mod owebsock;


const SLAB_START: usize = 1000000000;
const MAX_OUTPUT_CONNECTIONS: usize = 4096;
const HANDSHAKE_TIMEOUT: u64 = 30000;
const MESSAGE_TIMEOUT: u64 = 15000;
const GARBAGE_COLLECTOR_INTERVAL: u64 = 60_000;
const SUBSCRIPTION_LIFETIME: i64 = 3 * 60_000;
const DATA_POINTS: usize = 150;  // five minutes


pub type PeerHolder = Arc<RwLock<Peers>>;


#[allow(unused)] // start_time will be used later
pub struct Peers {
    touch_time: SteadyTime,
    gc_timer: Timeout,
    pub connected: usize,
    pub addresses: HashMap<SocketAddr, Token>,
    pub peers: Slab<Peer>,
    subscriptions: HashMap<RawRule, SteadyTime>,
}

#[derive(RustcDecodable, RustcEncodable)]
pub struct HostStats {
    addr: String,
    values: RawResult,
}

pub struct Peer {
    pub addr: SocketAddr,
    connection: Option<WebSocket>,
    timeout: Timeout,
    history: History,
    pub last_beacon: Option<(u64, Beacon)>,
}

impl Peer {
    pub fn connected(&self) -> bool {
        self.connection.as_ref().map(|x| !x.handshake).unwrap_or(false)
    }
}


pub fn ensure_started(ctx: &mut Context) {
    if let Some(peers) = ctx.deps.get::<PeerHolder>() {
        peers.write().unwrap().touch_time = SteadyTime::now();
        return; // already started
    }
    let range = Range::new(5, 150);
    let mut rng = thread_rng();
    let peers:Vec<_>;
    peers = ctx.deps.read::<GossipStats>().peers.keys().cloned().collect();
    let mut data = Peers {
        touch_time: SteadyTime::now(),
        peers: Slab::new_starting_at(Token(SLAB_START),
                                     MAX_OUTPUT_CONNECTIONS),
        gc_timer: ctx.eloop.timeout_ms(RemoteCollectGarbage,
            GARBAGE_COLLECTOR_INTERVAL).unwrap(),
        connected: 0,
        addresses: HashMap::new(),
        subscriptions: HashMap::new(),
    };
    for addr in peers {
        if let Some(tok) = data.peers.insert_with(|tok| Peer {
            addr: addr,
            last_beacon: None,
            connection: None,
            history: History::new(),
            timeout: ctx.eloop.timeout_ms(ReconnectPeer(tok),
                range.ind_sample(&mut rng)).unwrap(),
        }) {
            data.addresses.insert(addr, tok);
        } else {
            error!("Too many peers");
        }
    }
    ctx.deps.insert(Arc::new(RwLock::new(data)));
}

pub fn add_peer(addr: SocketAddr, ctx: &mut Context) {
    let range = Range::new(5, 150);
    let mut rng = thread_rng();
    if ctx.deps.get::<PeerHolder>().is_none() {
        // Remote handling is not enabled ATM
        return;
    }
    let mut data = ctx.deps.write::<Peers>();
    if data.addresses.contains_key(&addr) {
        return;
    }
    let ref mut eloop = ctx.eloop;
    if let Some(tok) = data.peers.insert_with(|tok| Peer {
        addr: addr,
        last_beacon: None,
        connection: None,
        timeout: eloop.timeout_ms(ReconnectPeer(tok),
            range.ind_sample(&mut rng)).unwrap(),
        history: History::new(),
    }) {
        data.addresses.insert(addr, tok);
    } else {
        error!("Too many peers");
    }
}

pub fn reconnect_peer(tok: Token, ctx: &mut Context) {
    let mut data = ctx.deps.write::<Peers>();
    if let Some(ref mut peer) = data.peers.get_mut(tok) {
        assert!(peer.connection.is_none());
        let range = Range::new(1000, 2000);
        let mut rng = thread_rng();
        if let Ok(conn) = WebSocket::connect(peer.addr) {
            match conn.register(tok, ctx.eloop) {
                Ok(_) => {
                    peer.connection = Some(conn);
                    peer.timeout = ctx.eloop.timeout_ms(ResetPeer(tok),
                        HANDSHAKE_TIMEOUT).unwrap();
                }
                _ => {
                    peer.connection = None;
                    peer.timeout = ctx.eloop.timeout_ms(ReconnectPeer(tok),
                        range.ind_sample(&mut rng)).unwrap();
                }
            }
        } else {
            peer.connection = None;
            peer.timeout = ctx.eloop.timeout_ms(ReconnectPeer(tok),
                range.ind_sample(&mut rng)).unwrap();
        }
    }
}

pub fn reset_peer(tok: Token, ctx: &mut Context) {
    let mut data = ctx.deps.write::<Peers>();
    if let Some(ref mut peer) = data.peers.get_mut(tok) {
        let wsock = replace(&mut peer.connection, None)
            .expect("No socket to reset");
        ctx.eloop.remove(&wsock.sock);
        let range = Range::new(1000, 2000);
        let mut rng = thread_rng();
        peer.timeout = ctx.eloop.timeout_ms(ReconnectPeer(tok),
            range.ind_sample(&mut rng)).unwrap();
    }
}

pub fn try_io(tok: Token, ev: EventSet, ctx: &mut Context) -> bool
{
    let dataref = ctx.deps.get::<PeerHolder>().unwrap().clone();
    let mut dataguard = dataref.write().unwrap();
    let ref mut data = dataguard.deref_mut();
    if let Some(ref mut peer) = data.peers.get_mut(tok) {
        let to_close = {
            let ref mut wsock = peer.connection.as_mut()
                .expect("Can't read from non-existent socket");
            let old = wsock.handshake;
            let mut to_close;
            if let Some(messages) = wsock.events(ev, tok, ctx) {
                if messages.len() > 0 {
                    assert!(ctx.eloop.clear_timeout(peer.timeout));
                    peer.timeout = ctx.eloop.timeout_ms(ResetPeer(tok),
                        MESSAGE_TIMEOUT).unwrap();
                }
                for msg in messages {
                    match msg {
                        InputMessage::Beacon(b) => {
                            peer.last_beacon = Some((time_ms(), b));
                        }
                        InputMessage::NewPeer(pstr) => {
                            // TODO(tailhook) process it
                            debug!("New peer from websock {:?}", pstr);
                            pstr.parse()
                            .map(|addr| ctx.deps.get::<Sender<p2p::Command>>()
                                        .unwrap().send(
                                            p2p::Command::AddGossipHost(addr)
                                        ).unwrap())
                            .map_err(|_|
                                error!("Bad host addr in NewPeer: {:?}", pstr))
                            .ok();
                        }
                        InputMessage::Stats(stats) => {
                            debug!("New stats from peer {:?}", stats);
                            peer.history.update_chunks(stats);
                        }
                    }
                }
                to_close = false;
            } else {
                to_close = true;
            }
            if old &&  !to_close && !wsock.handshake {
                data.connected += 1;
                assert!(ctx.eloop.clear_timeout(peer.timeout));
                peer.timeout = ctx.eloop.timeout_ms(ResetPeer(tok),
                    MESSAGE_TIMEOUT).unwrap();
                if data.subscriptions.len() > 0 {
                    for rule in data.subscriptions.keys() {
                        let subscr = OutputMessage::Subscribe(
                            rule.clone(), DATA_POINTS);
                        let msg = json::encode(&subscr).unwrap();
                        write_text(&mut wsock.output, &msg);
                    }
                    ctx.eloop.modify(&wsock.sock, tok, true, true);
                }
            } else if !old && to_close {
                data.connected -= 1;
            }
            to_close
        };
        if to_close {
            let range = Range::new(5, 150);
            let mut rng = thread_rng();
            peer.connection = None;
            assert!(ctx.eloop.clear_timeout(peer.timeout));
            peer.timeout = ctx.eloop.timeout_ms(ReconnectPeer(tok),
                    range.ind_sample(&mut rng)).unwrap();
        }
        return true;
    } else {
        return false;
    }
    // unreachable
    //data.peers.remove(tok)
    //return true;
}

pub fn serve_query_raw(req: &Request, context: &mut Context)
    -> Result<http::Response, Box<http::Error>>
{
    from_utf8(&req.body)
    .map_err(|_| BadRequest::err("Bad utf-8 encoding"))
    .and_then(|s| json::decode::<RawQuery>(s)
    .map_err(|_| BadRequest::err("Failed to decode query")))
    .and_then(|query| {
        ensure_started(context);

        let mut peerguard = context.deps.write::<Peers>();
        let mut peers = &mut *peerguard;
        let response: Vec<_> = peers.peers.iter().map(|peer| HostStats {
            addr: peer.addr.to_string(),
            values: rules::query_raw(query.rules.iter(),
                              DATA_POINTS, &peer.history),
        }).collect();

        for rule in query.rules.into_iter() {
            let ts = SteadyTime::now();
            if let Some(ts_ref) = peers.subscriptions.get_mut(&rule) {
                *ts_ref = ts;
                continue;
            }
            // TODO(tailhook) may optimize this rule.clone()
            let subscr = OutputMessage::Subscribe(rule.clone(), DATA_POINTS);
            let msg = json::encode(&subscr).unwrap();
            let ref mut addresses = &mut peers.addresses;
            let ref mut peerlist = &mut peers.peers;
            let ref mut eloop = context.eloop;
            for tok in addresses.values() {
                peerlist.replace_with(*tok, |mut peer| {
                    if let Some(ref mut wsock) = peer.connection {
                        if wsock.output.len() > MAX_OUTPUT_BUFFER {
                            debug!("Websocket buffer overflow");
                            eloop.remove(&wsock.sock);
                            return None;
                        }
                        let start = wsock.output.len() == 0;
                        write_text(&mut wsock.output, &msg);
                        if start {
                            eloop.modify(&wsock.sock, *tok, true, true);
                        }
                    }
                    Some(peer)
                }).unwrap()
            }
            peers.subscriptions.insert(rule, ts);
        }

        Ok(http::Response::json(&response))
    })
}

pub fn garbage_collector(ctx: &mut Context) {
    debug!("Garbage collector");
    let mut peers = ctx.deps.write::<Peers>();

    let cut_off = SteadyTime::now() - Duration::milliseconds(
        SUBSCRIPTION_LIFETIME);
    peers.subscriptions = replace(&mut peers.subscriptions, HashMap::new())
        .into_iter()
        .filter(|&(_, timestamp)| timestamp > cut_off)
        .collect();

    for peer in peers.peers.iter_mut() {
        peer.history.truncate_by_num(DATA_POINTS);
    }

    peers.gc_timer = ctx.eloop.timeout_ms(RemoteCollectGarbage,
        GARBAGE_COLLECTOR_INTERVAL).unwrap();
}
