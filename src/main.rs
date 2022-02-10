#[allow(unused_imports)]
use async_std::prelude::*;

mod gpio;
mod signal;
mod spi;

pub type EResult<T> = Result<T, Box<dyn std::error::Error>>;

use std::sync::{atomic::AtomicBool, Arc};

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
    let halt = Arc::new(AtomicBool::new(false));

    let (sig_rx, sig_hdl) = signal::run(halt.clone()).await?; // シグナルハンドラを起動
    let led_hdl = gpio::run(halt).await?; // LEDタスクを起動
    let spi_hdl = spi::run(sig_rx).await?; // SPIタスクを起動

    // graceful shutdown
    spi_hdl.await; // SPIタスクの終了を待機
    led_hdl.await; // LEDタスクの終了を待機
    sig_hdl.await; // シグナルハンドラの終了を待機

    Ok(())
}
