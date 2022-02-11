mod gpio;
mod i2c;
mod signal;
mod spi;

#[allow(unused_imports)]
use async_std::prelude::*;

use std::sync::{atomic::AtomicU64, Arc};

pub type EResult<T> = Result<T, Box<dyn std::error::Error>>;

#[macro_export]
macro_rules! perror {
    ($($args: expr),*) => {
        {
            print!("error: file: {}, line: {}", file!(), line!());
            $( print!(", {}: {}", stringify!($args), $args); )*
            println!(""); // to get a new line at the end
        }
    }
}

#[async_std::main]
async fn main() -> EResult<()> {
    let temp = Arc::new(AtomicU64::new(0)); // 気温
    let bright = Arc::new(AtomicU64::new(0)); // 明るさ

    let (sig_rx, sig_hdl) = signal::run().await?; // シグナルハンドラを起動
    let led_hdl = gpio::run(sig_rx.clone()).await?; // LEDタスクを起動
    let spi_hdl = spi::run(sig_rx.clone(), bright.clone()).await?; // SPIタスクを起動
    let i2c_hdl = i2c::run(sig_rx, temp, bright).await?; // I2Cタスクを起動

    // graceful shutdown
    i2c_hdl.await; // I2Cタスクの終了を待機
    spi_hdl.await; // SPIタスクの終了を待機
    led_hdl.await; // LEDタスクの終了を待機
    sig_hdl.await; // シグナルハンドラの終了を待機

    Ok(())
}
