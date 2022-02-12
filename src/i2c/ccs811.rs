#[allow(unused_imports)]
use async_std::prelude::*;

use super::Runner;
use crate::{perror, EResult};
use async_std::{
    channel::{Receiver, Sender},
    future::timeout,
    sync::Mutex,
    task::{self, JoinHandle},
};
use bitflags::bitflags;
use rppal::{gpio::OutputPin, i2c::I2c};
use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    time::Duration,
};

bitflags! {
    struct Status: u8 {
        const FW_START   = 0b10000000; // 0: ブートモード、1: アプリケーションモード（読み込み可能）
        const APP_VALID  = 0b00010000; // 0: ファームウェアのロード失敗、1: 有効
        const DATA_READY = 0b00001000; // 0: 新規データ無し、1: 新規データあり
        const ERROR      = 0b00000001; // 0: 正常、1: エラー
    }
}

pub(super) struct CCS811 {
    sig_rx: Receiver<()>,
    ccs811_pin: OutputPin,
}

impl CCS811 {
    const ADDR: u16 = 0x5a;
    const HW_ID: u8 = 0x81;
    const MODE: u8 = 0b00110000; // 60秒ごとに測定。割り込み無し

    const REG_STATUS: u8 = 0;
    const REG_MEAS_MODE: u8 = 1;
    const REG_ALG_RESULT_DATA: u8 = 2;
    const REG_HW_ID: u8 = 0x20;
    const REG_ERROR_ID: u8 = 0xe0;
    const REG_APP_START: u8 = 0xf4;

    pub(super) fn new(sig_rx: Receiver<()>, ccs811_pin: OutputPin) -> Self {
        CCS811 { sig_rx, ccs811_pin }
    }

    async fn init(&mut self, bus: &Arc<Mutex<I2c>>) -> EResult<()> {
        self.ccs811_pin.set_low();
        task::sleep(Duration::from_micros(100)).await;

        {
            let mut guard = bus.lock().await;
            if let Err(e) = guard.set_slave_address(Self::ADDR) {
                perror!(e);
            }

            let hw_id = self.get_hw_id(&guard)?;
            if hw_id != Self::HW_ID {
                return Err(format!("CCS811: invalid HW ID {hw_id}").into());
            }

            let status = self.get_status(&guard)?;
            println!("status = {:?}", status);
            if (status & Status::APP_VALID) != Status::APP_VALID {
                return Err("CCS811: invalid status".into());
            } else if (status & Status::FW_START) != Status::FW_START {
                self.set_mode(&guard)?; // 60秒ごとに測定
                ()
            }
        }

        task::sleep(Duration::from_micros(100)).await;

        {
            let mut guard = bus.lock().await;
            if let Err(e) = guard.set_slave_address(Self::ADDR) {
                perror!(e);
            }

            guard.smbus_send_byte(Self::REG_APP_START)?;
        }

        task::sleep(Duration::from_micros(100)).await;

        {
            let mut guard = bus.lock().await;
            if let Err(e) = guard.set_slave_address(Self::ADDR) {
                perror!(e);
            }

            let status = self.get_status(&guard)?;
            println!("status = {:?}", status);

            self.set_mode(&guard)?; // 60秒ごとに測定

            let mode = self.get_mode(&guard)?;
            println!("mode = 0b{:0b}", mode);
        }

        task::sleep(Duration::from_micros(50)).await;
        self.ccs811_pin.set_high();

        Ok(())
    }

    fn set_mode(&self, bus: &I2c) -> EResult<()> {
        bus.smbus_write_byte(Self::REG_MEAS_MODE, Self::MODE)?;
        Ok(())
    }

    fn get_status(&self, bus: &I2c) -> EResult<Status> {
        let status = bus.smbus_read_byte(Self::REG_STATUS)?;
        Ok(Status::from_bits(status).unwrap())
    }

    fn get_mode(&self, bus: &I2c) -> EResult<u8> {
        let mode = bus.smbus_read_byte(Self::REG_MEAS_MODE)?;
        Ok(mode)
    }

    fn get_hw_id(&self, bus: &I2c) -> EResult<u8> {
        let id = bus.smbus_read_byte(Self::REG_HW_ID)?;
        Ok(id)
    }

    fn get_error(&self, bus: &I2c) -> EResult<u8> {
        let err = bus.smbus_read_byte(Self::REG_ERROR_ID)?;
        Ok(err)
    }
}

impl Runner for CCS811 {
    fn run(mut self, bus: Arc<Mutex<I2c>>) -> EResult<JoinHandle<()>> {
        let wsec = Duration::from_secs(3);

        let f = async move {
            if let Err(e) = self.init(&bus).await {
                perror!(e);
                return;
            }

            loop {
                // タイムアウトかシグナルでの終了を待つ
                if timeout(wsec, self.sig_rx.recv()).await.is_ok() {
                    println!("exiting CCS811 ...");
                    break;
                }
            }
        };

        Ok(task::spawn(f))
    }
}
