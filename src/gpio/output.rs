#[allow(unused_imports)]
use async_std::prelude::*;

use super::Runner;
use crate::{perror, EResult};
use async_std::{
    channel::Receiver,
    task::{self, JoinHandle},
};
use futures::{select, FutureExt};
use rppal::gpio::{Level, Pin};

pub(super) struct Output {
    sig_rx: Receiver<()>,
    sw_rx: Receiver<Level>,
}

impl Output {
    pub(super) fn new(sig_rx: Receiver<()>, sw_rx: Receiver<Level>) -> Self {
        Output { sig_rx, sw_rx }
    }
}

impl Runner for Output {
    fn run(self, pin: Pin) -> EResult<JoinHandle<()>> {
        let mut pin = pin.into_output();

        let f = async move {
            loop {
                let mut sig_rx = self.sig_rx.recv().fuse();
                let mut sw_rx = self.sw_rx.recv().fuse();

                select!(
                    _ = sig_rx => {
                        // 終了シグナルを受信
                        println!("exiting GPIO Output ...");
                        break;
                    },
                    level = sw_rx => {
                        // LEDをオン・オフ
                        match level {
                            Ok(Level::High) => pin.set_high(),
                            Ok(Level::Low) => pin.set_low(),
                            Err(e) => {
                                perror!(e);
                                break;
                            }
                        }
                    }
                )
            }
        };

        Ok(task::spawn(f))
    }
}
