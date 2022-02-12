#[allow(unused_imports)]
use async_std::prelude::*;

use super::EResult;
use async_std::{
    channel::{Receiver, Sender},
    sync::Mutex,
    task::{self, JoinHandle},
};
use rppal::{gpio::OutputPin, i2c::I2c};
use std::sync::{atomic::AtomicU64, Arc};

mod adt7410;
mod ccs811;
mod st7032;

trait Runner {
    fn run(self, bus: Arc<Mutex<I2c>>) -> EResult<JoinHandle<()>>;
}

pub async fn run(
    sig_rx: Receiver<()>,
    ccs811_pin: OutputPin,
    temp: Arc<AtomicU64>,
    bright: Arc<AtomicU64>,
) -> EResult<JoinHandle<()>> {
    let bus = Arc::new(Mutex::new(I2c::new()?));

    // ディスプレイ
    let display = st7032::ST7032::new(sig_rx.clone(), temp.clone(), bright)
        .init(&bus)
        .await?;

    let task_display = display.run(bus.clone())?;

    // 温度センサ
    let task_adt7410 = adt7410::ADT7410::new(sig_rx.clone(), temp).run(bus.clone())?;

    // 環境センサ
    let task_ccs811 = ccs811::CCS811::new(sig_rx, ccs811_pin).run(bus)?;

    let hdl = task::spawn(async move {
        task_adt7410.await;
        task_display.await;
        task_ccs811.await;
    });

    Ok(hdl)
}
