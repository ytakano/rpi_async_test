#[allow(unused_imports)]
use async_std::prelude::*;

use super::{perror, EResult};
use async_std::{
    channel::Receiver,
    future::timeout,
    sync::Mutex,
    task::{self, JoinHandle},
};
use rppal::i2c::I2c;
use std::sync::Arc;
use std::time::Duration;

const ADT7410_ADDR: u16 = 0x48;
const ADT7410_REG: u8 = 0;

const ST7032_ADDR: u16 = 0x3e;
const ST7032_REG_SETTING: u8 = 0;
const ST7032_REG_DISPLAY: u8 = 0x40;
const ST7032_CONTRASTS: u16 = 32;

pub async fn run(rx: Receiver<()>) -> EResult<JoinHandle<()>> {
    let bus = Arc::new(Mutex::new(I2c::new()?));

    init_st7032(ST7032_CONTRASTS, &bus).await?;

    let task_adt7410 = task::spawn(run_adt7410(rx.clone(), bus.clone()));

    print_st7032(&bus, b"Hello").await?;
    newline_st7032(&bus).await?;
    print_st7032(&bus, b"World").await?;

    let hdl = task::spawn(async move {
        task_adt7410.await;
    });

    Ok(hdl)
}

async fn run_adt7410(rx: Receiver<()>, bus: Arc<Mutex<I2c>>) {
    let wsec = Duration::from_secs(2);

    loop {
        // タイムアウトかシグナルでの終了を待つ
        if timeout(wsec, rx.recv()).await.is_ok() {
            println!("exiting SPI ...");
            break;
        }

        {
            let mut guard = bus.lock().await;
            if let Err(e) = guard.set_slave_address(ADT7410_ADDR) {
                perror!(e);
            }

            match guard.smbus_read_word(ADT7410_REG) {
                Ok(n) => {
                    let celciuls = (n.to_be() >> 3) as f64 / 16.0;
                    println!("ADT7410: {celciuls} 度");
                }
                Err(e) => {
                    perror!(e);
                }
            }
        }
    }
}

async fn init_st7032(contrasts: u16, bus: &Arc<Mutex<I2c>>) -> EResult<()> {
    let lower = contrasts & 0x0f;
    let upper = (contrasts & 0x30) >> 4;
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
        guard.set_slave_address(ST7032_ADDR)?;
        guard.smbus_block_write(ST7032_REG_SETTING, &v)?;
    }

    task::sleep(Duration::from_millis(200)).await;

    let v: [u8; 3] = [0x38, 0x0d, 0x01];
    {
        let mut guard = bus.lock().await;
        guard.set_slave_address(ST7032_ADDR)?;
        guard.smbus_block_write(ST7032_REG_SETTING, &v)?;
    }

    task::sleep(Duration::from_millis(1)).await;

    Ok(())
}

async fn clear_st7032(bus: &Arc<Mutex<I2c>>) -> EResult<()> {
    {
        let mut guard = bus.lock().await;
        guard.set_slave_address(ST7032_ADDR)?;
        guard.smbus_write_byte(ST7032_REG_SETTING, 0x01)?;
    }
    task::sleep(Duration::from_millis(1)).await;
    Ok(())
}

async fn putc_st7032(bus: &Arc<Mutex<I2c>>, c: u8) -> EResult<()> {
    let c = if c < 0x06 { 0x20 } else { c };
    let mut guard = bus.lock().await;
    guard.set_slave_address(ST7032_ADDR)?;
    guard.smbus_write_byte(ST7032_REG_DISPLAY, c)?;
    Ok(())
}

async fn print_st7032(bus: &Arc<Mutex<I2c>>, line: &[u8]) -> EResult<()> {
    let mut guard = bus.lock().await;
    guard.set_slave_address(ST7032_ADDR)?;
    for c in line {
        let c = if *c < 0x06 { 0x20 } else { *c };
        guard.smbus_write_byte(ST7032_REG_DISPLAY, c)?;
    }
    Ok(())
}

async fn newline_st7032(bus: &Arc<Mutex<I2c>>) -> EResult<()> {
    {
        let mut guard = bus.lock().await;
        guard.set_slave_address(ST7032_ADDR)?;
        guard.smbus_write_byte(ST7032_REG_SETTING, 0xc0)?;
    }
    task::sleep(Duration::from_millis(1)).await;
    Ok(())
}
