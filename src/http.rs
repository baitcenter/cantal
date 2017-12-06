use std::time::Duration;
use std::process::exit;

use failure::Error;
use ns_env_config::Router as NsRouter;
use futures::{Future, Stream};
use tk_easyloop::{handle, spawn};
use tk_http;
use tk_http::server::{Proto};
use tk_listen::{BindMany, ListenExt};

use frontend;


pub fn spawn_listener(ns: &NsRouter, host: &str, port: u16, localhost: bool)
    -> Result<(), Error>
{
    let hcfg = tk_http::server::Config::new()
        .inflight_request_limit(2)
        .inflight_request_prealoc(0)
        .first_byte_timeout(Duration::new(10, 0))
        .keep_alive_timeout(Duration::new(600, 0))
        .headers_timeout(Duration::new(1, 0))             // no big headers
        .input_body_byte_timeout(Duration::new(1, 0))     // no big bodies
        .input_body_whole_timeout(Duration::new(2, 0))
        .output_body_byte_timeout(Duration::new(1, 0))
        .output_body_whole_timeout(Duration::new(30, 0))
        .done();

    let mut addr = vec![host];
    if localhost {
        addr.push("localhost");
    }
    let host = host.to_string();
    spawn(BindMany::new(ns.subscribe_many(addr, port)
                        .map(|addr| addr.addresses_at(0)), &handle())
        .sleep_on_error(Duration::from_millis(100), &handle())
        .map(move |(socket, saddr)| {
            Proto::new(socket, &hcfg,
                frontend::Dispatcher { },
                &handle())
            .map_err(move |e| {
                debug!("Http protocol error for {}: {}", saddr, e);
            })
        })
        .listen(500)
        .then(move |res| -> Result<(), ()> {
            error!("Listener {}:{} exited: {:?}", host, port, res);
            exit(3);
        }));
    Ok(())
}
