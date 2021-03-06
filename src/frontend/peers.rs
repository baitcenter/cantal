use std::sync::{Arc};

use frontend::{Request};
use frontend::routing::Format;
use frontend::quick_reply::{reply, respond};
use gossip::{Peer, Gossip};


#[derive(Serialize)]
struct Peers {
    peers: Vec<Arc<Peer>>,
}

pub fn serve<S: 'static>(gossip: &Gossip, format: Format)
    -> Request<S>
{
    let gossip = gossip.clone();
    reply(move |e| {
        Box::new(respond(e, format,
            &Peers {
                peers: gossip.get_peers(),
            }
        ))
    })
}

pub fn serve_only_remote<S: 'static>(gossip: &Gossip, format: Format)
    -> Request<S>
{
    let gossip = gossip.clone();
    reply(move |e| {
        Box::new(respond(e, format,
            &Peers {
                peers: gossip.get_peers().into_iter().filter(|x| {
                    x.report.as_ref()
                        .map(|&(_, ref r)| r.has_remote)
                        .unwrap_or(false)
                }).collect(),
            }
        ))
    })
}
