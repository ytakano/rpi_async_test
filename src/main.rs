#[macro_use]
extern crate diesel;

mod db;
mod gpio;
mod i2c;
mod schema;
mod signal;
mod spi;

#[allow(unused_imports)]
use async_std::prelude::*;

use std::sync::{
    atomic::{AtomicU16, AtomicU64},
    Arc,
};

pub type EResult<T> = Result<T, Box<dyn std::error::Error>>;

#[macro_export]
macro_rules! perror {
    ($($args: expr),*) => {
        {
            eprint!("error: file: {}, line: {}", file!(), line!());
            $( eprint!(", {}: {}", stringify!($args), $args); )*
            eprintln!(""); // to get a new line at the end
        }
    }
}

#[derive(Clone, Debug)]
pub struct Air {
    pub temp: Arc<AtomicU64>, // 気温
    pub co2: Arc<AtomicU16>,  // 二酸化炭素濃度
    pub tvoc: Arc<AtomicU16>, // 総揮発性有機化合物
}

impl Air {
    fn new() -> Self {
        Air {
            temp: Default::default(),
            co2: Default::default(),
            tvoc: Default::default(),
        }
    }
}

#[async_std::main]
async fn main() -> EResult<()> {
    let bright = Arc::new(AtomicU64::new(0)); // 明るさ
    let air = Air::new();

    let (sig_rx, sig_hdl) = signal::run().await?; // シグナルハンドラを起動
    let (led_hdl, ccs811_pin) = gpio::run(sig_rx.clone()).await?; // LEDタスクを起動
    let spi_hdl = spi::run(sig_rx.clone(), bright.clone()).await?; // SPIタスクを起動
    let i2c_hdl = i2c::run(sig_rx, ccs811_pin, air.clone(), bright.clone()).await?; // I2Cタスクを起動
    let _ = db::run(air, bright);

    // graceful shutdown
    i2c_hdl.await; // I2Cタスクの終了を待機
    spi_hdl.await; // SPIタスクの終了を待機
    led_hdl.await; // LEDタスクの終了を待機
    sig_hdl.await; // シグナルハンドラの終了を待機

    Ok(())
}
