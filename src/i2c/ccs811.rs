#[allow(unused_imports)]
use async_std::prelude::*;

use super::Runner;
use crate::{perror, Air, EResult};
use async_std::{
    channel::Receiver,
    future::timeout,
    sync::Mutex,
    task::{self, JoinHandle},
};
use bitflags::bitflags;
use rppal::{gpio::OutputPin, i2c::I2c};
use std::{
    sync::{atomic::Ordering, Arc},
    time::Duration,
};

const ADDR: u16 = 0x5a;

bitflags! {
    struct Status: u8 {
        const FW_START   = 0b1000_0000; // 0: ブートモード、1: アプリケーションモード（読み込み可能）
        const APP_VALID  = 0b0001_0000; // 0: ファームウェアのロード失敗、1: 有効
        const DATA_READY = 0b0000_1000; // 0: 新規データ無し、1: 新規データあり
        const ERROR      = 0b0000_0001; // 0: 正常、1: エラー
        const RESERVED0  = 0b0100_0000;
        const RESERVED1  = 0b0010_0000;
        const RESERVED2  = 0b0000_0100;
        const RESERVED3  = 0b0000_0010;
    }

    struct Error: u8 {
        const HEATER_SUPPLY     = 0b0010_0000;
        const HEATER_FAULT      = 0b0001_0000;
        const MOX_RESISTANCE    = 0b0000_1000;
        const MEAS_MODE_INVALID = 0b0000_0100;
        const READ_REG_INVALID  = 0b0000_0010;
        const MSG_INVALID       = 0b0000_0001;
        const RESERVED0         = 0b1000_0000;
        const RESERVED1         = 0b0100_0000;
    }
}

pub(super) struct CCS811 {
    sig_rx: Receiver<()>,
    ccs811_pin: OutputPin,
    air: Air,
}

struct WakeGuard<'a> {
    ccs811_pin: &'a mut OutputPin,
}

impl<'a> WakeGuard<'a> {
    const HW_ID: u8 = 0x81;
    const MODE: u8 = 0b00010000; // 1秒ごとに測定。割り込み無し

    const REG_STATUS: u8 = 0;
    const REG_MEAS_MODE: u8 = 1;
    const REG_ALG_RESULT_DATA: u8 = 2;
    const REG_HW_ID: u8 = 0x20;
    const REG_ERROR_ID: u8 = 0xe0;
    const REG_APP_START: u8 = 0xf4;

    async fn new(ccs811_pin: &'a mut OutputPin) -> WakeGuard<'a> {
        ccs811_pin.set_low();
        task::sleep(Duration::from_micros(100)).await;
        WakeGuard { ccs811_pin }
    }

    fn set_mode(&self, bus: &I2c) -> EResult<()> {
        bus.smbus_write_byte(Self::REG_MEAS_MODE, Self::MODE)?;
        Ok(())
    }

    fn get_status(&self, bus: &I2c) -> EResult<Status> {
        let status = bus.smbus_read_byte(Self::REG_STATUS)?;
        match Status::from_bits(status) {
            Some(s) => Ok(s),
            None => Err(format!("CCS811: error: status = 0b{:0b}", status).into()),
        }
    }

    fn get_mode(&self, bus: &I2c) -> EResult<u8> {
        let mode = bus.smbus_read_byte(Self::REG_MEAS_MODE)?;
        Ok(mode)
    }

    fn get_hw_id(&self, bus: &I2c) -> EResult<u8> {
        let id = bus.smbus_read_byte(Self::REG_HW_ID)?;
        Ok(id)
    }

    fn get_error(&self, bus: &I2c) -> EResult<Error> {
        let err = bus.smbus_read_byte(Self::REG_ERROR_ID)?;
        Ok(Error::from_bits(err).unwrap())
    }

    fn get_data(&self, bus: &I2c) -> EResult<Option<(u16, u16)>> {
        let status = self.get_status(bus)?;
        if (status & Status::DATA_READY) != Status::DATA_READY {
            return Ok(None);
        }

        let mut buf: [u8; 8] = [0; 8];
        bus.block_read(Self::REG_ALG_RESULT_DATA, &mut buf)?;
        let co2 = ((buf[0] as u16) << 8) | (buf[1] as u16);
        let tvoc = ((buf[2] as u16) << 8) | (buf[3] as u16);

        self.print_error(Status::from_bits(buf[4]).unwrap(), bus);

        Ok(Some((co2, tvoc)))
    }

    fn print_error(&self, status: Status, bus: &I2c) {
        if (status & Status::ERROR) != Status::ERROR {
            return;
        }

        if let Ok(e) = self.get_error(bus) {
            eprintln!("CCS811: error: {:?}", e);
        }
    }

    async fn init(&mut self, bus: &Arc<Mutex<I2c>>) -> EResult<()> {
        {
            let mut guard = bus.lock().await;
            if let Err(e) = guard.set_slave_address(ADDR) {
                perror!(e);
            }

            println!("CCS811: checking HW ID");
            let hw_id = self.get_hw_id(&guard)?;
            if hw_id != Self::HW_ID {
                return Err(format!("CCS811: invalid HW ID {hw_id}").into());
            }

            println!("CCS811: checking status");
            let status = self.get_status(&guard)?;
            self.print_error(status, &guard);
            println!("status = {:?}", status);

            if (status & Status::APP_VALID) != Status::APP_VALID {
                return Err("CCS811: invalid status".into());
            }

            if (status & Status::FW_START) == Status::FW_START {
                // 内部アプリケーションは起動済み
                println!("CCS811: setting mode");

                // 動作モード設定
                self.set_mode(&guard)?; // 1秒ごとに測定
                task::sleep(Duration::from_micros(50)).await;
                return Ok(());
            }
        }

        task::sleep(Duration::from_micros(100)).await;

        {
            // 内部アプリケーションを起動
            let mut guard = bus.lock().await;
            if let Err(e) = guard.set_slave_address(ADDR) {
                perror!(e);
            }

            println!("CCS811: starting");
            guard.smbus_send_byte(Self::REG_APP_START)?;
        }

        task::sleep(Duration::from_micros(100)).await;

        {
            let mut guard = bus.lock().await;
            if let Err(e) = guard.set_slave_address(ADDR) {
                perror!(e);
            }

            // 内部アプリケーションの起動をチェック
            println!("CCS811: checking status again");
            let status = self.get_status(&guard)?;
            self.print_error(status, &guard);
            println!("status = {:?}", status);

            if (status & Status::FW_START) != Status::FW_START {
                // 内部アプリケーションの起動に失敗
                return Err("CCS811: failed to start FW".into());
            }

            // 動作モードを設定
            println!("CCS811: setting mode");
            self.set_mode(&guard)?; // 1秒ごとに測定

            let mode = self.get_mode(&guard)?;
            println!("CCS811: mode = 0b{:0b}", mode);
        }

        task::sleep(Duration::from_micros(50)).await;
        Ok(())
    }
}

impl<'a> Drop for WakeGuard<'a> {
    fn drop(&mut self) {
        self.ccs811_pin.set_high();
    }
}

impl CCS811 {
    pub(super) fn new(sig_rx: Receiver<()>, ccs811_pin: OutputPin, air: Air) -> Self {
        CCS811 {
            sig_rx,
            ccs811_pin,
            air,
        }
    }

    async fn wake_up<'a>(&mut self) -> WakeGuard<'_> {
        WakeGuard::new(&mut self.ccs811_pin).await
    }
}

impl Runner for CCS811 {
    fn run(mut self, bus: Arc<Mutex<I2c>>) -> EResult<JoinHandle<()>> {
        let wsec = Duration::from_secs(1);

        let f = async move {
            {
                // 初期化
                let mut wake = self.wake_up().await;
                if let Err(e) = wake.init(&bus).await {
                    perror!(e);
                    return;
                }
            }

            // 10分待機
            if timeout(Duration::from_secs(1), self.sig_rx.recv())
                .await
                .is_ok()
            {
                println!("exiting CCS811 ...");
                return;
            }

            loop {
                // タイムアウトかシグナルでの終了を待つ
                if timeout(wsec, self.sig_rx.recv()).await.is_ok() {
                    println!("exiting CCS811 ...");
                    break;
                }

                let co2_val;
                let tvoc_val;
                {
                    let wake = self.wake_up().await;
                    let mut guard = bus.lock().await;
                    if let Err(e) = guard.set_slave_address(ADDR) {
                        perror!(e);
                    }

                    match wake.get_data(&guard) {
                        Ok(Some((co2, tvoc))) => {
                            if co2 < 400 || co2 > 8192 || tvoc > 1187 {
                                continue;
                            }

                            println!("CCS811: CO2 = {co2}, TVOC = {tvoc}");
                            co2_val = co2;
                            tvoc_val = tvoc;
                        }
                        Ok(None) => continue,
                        Err(e) => {
                            perror!(e);
                            continue;
                        }
                    }
                }

                self.air.co2.store(co2_val, Ordering::Relaxed);
                self.air.tvoc.store(tvoc_val, Ordering::Relaxed);
            }
        };

        Ok(task::spawn(f))
    }
}
