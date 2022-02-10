#[allow(unused_imports)]
use async_std::prelude::*;

use super::{perror, EResult};
use async_std::{
    channel::{self, Receiver},
    task::{self, JoinHandle},
};
use signal_hook::consts::signal::*;
use signal_hook_async_std::Signals;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

const CHANNEL_SIZE: usize = 32;

pub async fn run(halt: Arc<AtomicBool>) -> EResult<(Receiver<()>, JoinHandle<()>)> {
    let mut signals = Signals::new(&[SIGHUP, SIGTERM, SIGINT, SIGQUIT])?;
    let (tx, rx) = channel::bounded(CHANNEL_SIZE);

    let f = async move {
        while let Some(signal) = signals.next().await {
            match signal {
                SIGHUP => {
                    // Reload configuration
                    // Reopen the log file
                }
                SIGTERM | SIGINT | SIGQUIT => {
                    // Shutdown the system;
                    halt.store(true, Ordering::Relaxed);
                    if let Err(e) = tx.send(()).await {
                        perror!(e);
                    }
                    println!("");
                    println!("exiting signal handler ...");
                    return;
                }
                _ => unreachable!(),
            }
        }
    };

    println!("initialized signal handler");

    Ok((rx, task::spawn(f)))
}
