#[allow(unused_imports)]
use async_std::prelude::*;

use super::Runner;
use crate::{perror, EResult};
use async_std::{
    channel::{Receiver, Sender, TryRecvError},
    task::{self, JoinHandle},
};
use rppal::gpio::{Level, Pin};
use std::time::Duration;

pub(super) struct Input {
    sig_rx: Receiver<()>,
    sw_tx: Sender<Level>,
}

impl Input {
    pub(super) fn new(sig_rx: Receiver<()>, sw_tx: Sender<Level>) -> Self {
        Input { sig_rx, sw_tx }
    }
}

impl Runner for Input {
    fn run(self, pin: Pin) -> EResult<JoinHandle<()>> {
        let t = Duration::from_millis(200);
        let mut pin = pin.into_input_pulldown();

        // GPIOの入力変化を送信
        let f = async move {
            loop {
                match pin.poll_interrupt(false, Some(t)) {
                    Ok(Some(level)) => {
                        let p = pin.pin();
                        println!("GPIO({p}): {level}");
                        if self.sw_tx.send(level).await.is_err() {
                            println!("exiting GPIO Input ...");
                            break;
                        }
                    }
                    Ok(None) => {
                        // timeout
                        match self.sig_rx.try_recv() {
                            Err(TryRecvError::Empty) => (),
                            _ => {
                                println!("exiting GPIO Input ...");
                                break;
                            }
                        }

                        // 現在のレベルを送信
                        let level = pin.read();
                        if let Err(e) = self.sw_tx.send(level).await {
                            perror!(e);
                            println!("exiting GPIO Input ...");
                            break;
                        }
                    }
                    Err(e) => {
                        perror!(e);
                        break;
                    }
                }
            }
        };

        Ok(task::spawn(f))
    }
}
