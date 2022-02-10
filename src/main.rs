#[allow(unused_imports)]
use async_std::prelude::*;

mod gpio;
mod msg;
mod signal;

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
    let (tx, rx, sig_hdl) = signal::run().await?; // シグナルハンドラを起動
    let led_hdl = gpio::run(tx, rx).await?; // LEDタスクを起動

    led_hdl.await; // LEDタスクの終了を待機
    sig_hdl.await; // シグナルハンドラの終了を待機

    Ok(())
}
