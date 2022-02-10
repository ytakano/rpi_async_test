use super::{msg::Msg, perror, EResult};
use async_std::{
    channel::{self, Receiver, Sender},
    task::{self, JoinHandle},
};
use signal_hook::consts::signal::*;
use signal_hook_async_std::Signals;

#[allow(unused_imports)]
use async_std::prelude::*;

const CHANNEL_SIZE: usize = 32;

pub async fn run() -> EResult<(Sender<Msg>, Receiver<Msg>, JoinHandle<()>)> {
    let mut signals = Signals::new(&[SIGHUP, SIGTERM, SIGINT, SIGQUIT])?;
    let (tx, rx) = channel::bounded(CHANNEL_SIZE);

    let tx2 = tx.clone();
    let f = async move {
        while let Some(signal) = signals.next().await {
            match signal {
                SIGHUP => {
                    // Reload configuration
                    // Reopen the log file
                }
                SIGTERM | SIGINT | SIGQUIT => {
                    // Shutdown the system;
                    if let Err(e) = tx.send(Msg::Quit).await {
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

    Ok((tx2, rx, task::spawn(f)))
}
