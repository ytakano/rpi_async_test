#[allow(unused_imports)]
use async_std::prelude::*;

use super::Runner;
use crate::{perror, EResult};
use async_std::{
    channel::Receiver,
    future::timeout,
    task::{self, JoinHandle},
};
use bitflags::bitflags;
use rppal::spi::{Bus, Mode, SlaveSelect, Spi};
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

bitflags! {
    struct MCP3208_0: u8 {
        const START = 0b00000100; // start bit
        const SGL = 0b00000010; // SGL (絶対値)
        const D2 =  0b00000001; // チャネル選択 (2ビット目)
    }

    struct MCP3208_1: u8 {
        const D1 = 0b10000000; // チャネル選択 (1ビット目)
        const D0 = 0b01000000; // チャネル選択 (0ビット目)
    }
}

pub(super) struct MCP3208 {
    sig_rx: Receiver<()>,
    bright: Arc<AtomicU64>,
    min: u16,
    max: u16,
}

impl MCP3208 {
    const CLOCK: u32 = 1000 * 1000; // 1 MHz

    pub(super) fn new(sig_rx: Receiver<()>, bright: Arc<AtomicU64>) -> Self {
        MCP3208 {
            sig_rx,
            bright,
            min: 500,
            max: 3800,
        }
    }
}

impl Runner for MCP3208 {
    fn run(mut self) -> EResult<JoinHandle<()>> {
        let s = Spi::new(Bus::Spi0, SlaveSelect::Ss0, Self::CLOCK, Mode::Mode0)?;

        let f = async move {
            let wsec = Duration::from_secs(1);
            let mut read_buf: [u8; 3] = [0; 3];
            let write_buf: [u8; 3] = [(MCP3208_0::START | MCP3208_0::SGL).bits, 0, 0];

            loop {
                // タイムアウトかシグナルでの終了を待つ
                if timeout(wsec, self.sig_rx.recv()).await.is_ok() {
                    println!("exiting MCP3208 ...");
                    break;
                }

                // MCP3208から読み込み
                match s.transfer(&mut read_buf, &write_buf) {
                    Ok(_size) => {
                        let val = ((read_buf[1] & 0b00001111) as u16) << 8 | read_buf[0] as u16;
                        self.min = if self.min > val { val } else { self.min };
                        self.max = if self.max < val { val } else { self.max };

                        let per = (val - self.min) as f64 / (self.max - self.min) as f64 * 100.0;
                        self.bright.store(per.to_bits(), Ordering::Relaxed); // 共有変数に保存
                        println!("MCP3208(0): {:.2} %", per);
                    }
                    Err(e) => {
                        perror!(e);
                    }
                }
            }
        };

        Ok(task::spawn(f))
    }
}
