#[allow(unused_imports)]
use async_std::prelude::*;

use super::{perror, EResult};
use async_std::{
    channel::Receiver,
    future::timeout,
    task::{self, JoinHandle},
};
use rppal::i2c::I2c;
use std::time::Duration;

const ADT7410_ADDR: u16 = 0x48;
const ADT7410_REG: u8 = 0;

pub async fn run(rx: Receiver<()>) -> EResult<JoinHandle<()>> {
    let mut bus = I2c::new()?;

    bus.set_slave_address(ADT7410_ADDR)?;

    let f = async move {
        let wsec = Duration::from_secs(2);

        loop {
            // タイムアウトかシグナルでの終了を待つ
            if timeout(wsec, rx.recv()).await.is_ok() {
                println!("exiting SPI ...");
                break;
            }

            match bus.smbus_read_word(ADT7410_REG) {
                Ok(n) => {
                    let celciuls = (n.to_be() >> 3) as f64 / 16.0;
                    println!("ADT7410: {celciuls} 度");
                }
                Err(e) => {
                    perror!(e);
                }
            }
        }
    };

    let hdl = task::spawn(f);
    Ok(hdl)
}
