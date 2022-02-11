#[allow(unused_imports)]
use async_std::prelude::*;

use super::Runner;
use crate::{perror, EResult};
use async_std::{
    channel::Receiver,
    future::timeout,
    sync::Mutex,
    task::{self, JoinHandle},
};
use rppal::i2c::I2c;
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

/// 温度センサーADT7410
pub(super) struct ADT7410 {
    sig_rx: Receiver<()>,
    temp: Arc<AtomicU64>, // 気温
}

impl ADT7410 {
    const ADDR: u16 = 0x48;
    const REG: u8 = 0;

    pub(super) fn new(sig_rx: Receiver<()>, temp: Arc<AtomicU64>) -> Self {
        ADT7410 { sig_rx, temp }
    }
}

impl Runner for ADT7410 {
    fn run(self, bus: Arc<Mutex<I2c>>) -> EResult<JoinHandle<()>> {
        let wsec = Duration::from_secs(1);

        let f = async move {
            loop {
                // タイムアウトかシグナルでの終了を待つ
                if timeout(wsec, self.sig_rx.recv()).await.is_ok() {
                    println!("exiting ADT7410 ...");
                    break;
                }

                {
                    let mut guard = bus.lock().await;
                    if let Err(e) = guard.set_slave_address(Self::ADDR) {
                        perror!(e);
                    }

                    match guard.smbus_read_word(Self::REG) {
                        Ok(n) => {
                            let celciuls = (n.to_be() >> 3) as f64 / 16.0;
                            self.temp.store(celciuls.to_bits(), Ordering::Relaxed); // 共有変数に保存
                            println!("ADT7410: {:.2} 度", celciuls);
                        }
                        Err(e) => {
                            perror!(e);
                        }
                    }
                }
            }
        };

        Ok(task::spawn(f))
    }
}
