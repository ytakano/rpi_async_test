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
    marker::PhantomData,
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

/// 液晶ディスプレイ ST7032
pub(super) struct ST7032<T> {
    sig_rx: Receiver<()>,
    temp: Arc<AtomicU64>,   // 温度
    bright: Arc<AtomicU64>, // 明るさ
    _state: PhantomData<T>, // 型状態
}

pub(super) struct Uninit {}
pub(super) struct Initialized {}

impl<T> ST7032<T> {
    const ADDR: u16 = 0x3e;
    const REG_SETTING: u8 = 0;
    const REG_DISPLAY: u8 = 0x40;
    const CONTRASTS: u16 = 32;
}

impl ST7032<Uninit> {
    pub(super) fn new(
        sig_rx: Receiver<()>,
        temp: Arc<AtomicU64>,
        bright: Arc<AtomicU64>,
    ) -> ST7032<Uninit> {
        ST7032 {
            sig_rx,
            temp,
            bright,
            _state: PhantomData,
        }
    }

    /// 初期化
    pub(super) async fn init(self, bus: &Arc<Mutex<I2c>>) -> EResult<ST7032<Initialized>> {
        let lower = Self::CONTRASTS & 0x0f;
        let upper = (Self::CONTRASTS & 0x30) >> 4;
        let v: [u8; 6] = [
            0x38,
            0x39,
            0x14,
            0x70 | lower as u8,
            0x54 | upper as u8,
            0x6c,
        ];

        {
            let mut guard = bus.lock().await;
            guard.set_slave_address(Self::ADDR)?;
            guard.smbus_block_write(Self::REG_SETTING, &v)?;
        }

        task::sleep(Duration::from_millis(200)).await;

        let v: [u8; 3] = [0x38, 0x0d, 0x01];
        {
            let mut guard = bus.lock().await;
            guard.set_slave_address(Self::ADDR)?;
            guard.smbus_block_write(Self::REG_SETTING, &v)?;
        }

        task::sleep(Duration::from_millis(1)).await;

        Ok(ST7032 {
            sig_rx: self.sig_rx,
            temp: self.temp,
            bright: self.bright,
            _state: PhantomData,
        })
    }
}

impl ST7032<Initialized> {
    /// ディスプレイクリア
    async fn clear(&self, bus: &Arc<Mutex<I2c>>) -> EResult<()> {
        {
            let mut guard = bus.lock().await;
            guard.set_slave_address(Self::ADDR)?;
            guard.smbus_write_byte(Self::REG_SETTING, 0x01)?;
        }
        task::sleep(Duration::from_millis(1)).await;
        Ok(())
    }

    /// 改行
    async fn newline(&self, bus: &Arc<Mutex<I2c>>) -> EResult<()> {
        {
            let mut guard = bus.lock().await;
            guard.set_slave_address(Self::ADDR)?;
            guard.smbus_write_byte(Self::REG_SETTING, 0xc0)?;
        }
        task::sleep(Duration::from_millis(1)).await;
        Ok(())
    }

    /// 2行表示
    async fn print(&self, line1: &str, line2: Option<&str>, bus: &Arc<Mutex<I2c>>) -> EResult<()> {
        self.clear(bus).await?;
        self.print_line(line1.as_bytes(), bus).await?;
        self.newline(bus).await?;
        if let Some(line) = line2 {
            self.print_line(line.as_bytes(), bus).await?;
        }

        Ok(())
    }

    /// 一行表示
    async fn print_line(&self, line: &[u8], bus: &Arc<Mutex<I2c>>) -> EResult<()> {
        let mut guard = bus.lock().await;
        guard.set_slave_address(Self::ADDR)?;
        for c in line {
            let c = if *c < 0x06 { 0x20 } else { *c };
            guard.smbus_write_byte(Self::REG_DISPLAY, c)?;
        }
        Ok(())
    }
}

impl Runner for ST7032<Initialized> {
    fn run(self, bus: Arc<Mutex<I2c>>) -> EResult<JoinHandle<()>> {
        let wsec = Duration::from_secs(1);

        let f = async move {
            if let Err(e) = self.print("init ...", None, &bus).await {
                perror!(e);
                return;
            }

            loop {
                // タイムアウトかシグナルでの終了を待つ
                if timeout(wsec, self.sig_rx.recv()).await.is_ok() {
                    println!("exiting ST7032 ...");
                    break;
                }

                let temp = format!("{:.2} C", f64::from_bits(self.temp.load(Ordering::Relaxed)));
                let bright = format!("{:.2} %", f64::from_bits(self.bright.load(Ordering::Relaxed)));

                if let Err(e) = self.print(&temp, Some(&bright), &bus).await {
                    perror!(e);
                    break;
                }
            }
        };

        Ok(task::spawn(f))
    }
}
