extern crate libc;
#[macro_use] extern crate log;
extern crate cbor;
extern crate argparse;
extern crate cantal;
extern crate rustc_serialize;

use std::env;
use std::thread;
use std::fs::File;
use std::path::PathBuf;
use std::sync::RwLock;

use rustc_serialize::Decodable;
use cbor::{Decoder};
use argparse::{ArgumentParser, Store, ParseOption};


mod aio;
mod util;
mod server;
mod stats;
mod staticfiles;
mod scanner;
mod scan;
mod mountpoints;
mod deltabuf;
mod history;
mod storage;


fn main() {
    let mut host = "127.0.0.1".to_string();
    let mut port = 22682u16;
    let mut storage_dir = None::<PathBuf>;
    {
        let mut ap = ArgumentParser::new();
        ap.refer(&mut port)
            .add_option(&["-p", "--port"], Store,
                "Port for http interface");
        ap.refer(&mut host)
            .add_option(&["-h", "--host"], Store,
                "Host for http interface (default 127.0.0.1)");
        ap.refer(&mut storage_dir)
            .add_option(&["-d", "--storage-dir"], ParseOption,
                "A directory to serialize data to");
        match ap.parse_args() {
            Ok(()) => {}
            Err(x) => {
                env::set_exit_status(x);
                return;
            }
        }
    }
    let stats = &RwLock::new(stats::Stats::new());
    let cell = &util::Cell::new();

    let _storage = storage_dir.as_ref().map(|path| {
        let result = File::open(&path.join("current.msgpack"))
            .map_err(|e| error!("Error reading old data: {}. Ignoring...", e))
            .and_then(|f| Decoder::from_reader(f).decode().next()
                .ok_or_else(|| error!(
                    "Error parsing old data: No data. Ignoring..."))
                .and_then(|r| r.map_err(|e| error!(
                    "Error parsing old data {:?}. Ignoring...", e)
                )));
        if let Ok(history) = result {
            stats.write().unwrap().history = history;
        }
        let path = path.clone();
        thread::scoped(move || {
            storage::storage_loop(cell, &path, stats)
        })
    });

    let _scan = thread::scoped(move || {
        scanner::scan_loop(stats, storage_dir.map(|_| cell))
    });

    match server::run_server(stats, host, port) {
        Ok(()) => {}
        Err(x) => {
            error!("Error running server: {}", x);
            env::set_exit_status(1);
            return;
        }
    }
}
