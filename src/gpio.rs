#[allow(unused_imports)]
use async_std::prelude::*;

use super::EResult;
use async_std::{
    channel::{self, Receiver},
    task::{self, JoinHandle},
};
use rppal::gpio::{Gpio, Pin};

mod input;
mod output;

trait Runner {
    fn run(self, pin: Pin) -> EResult<JoinHandle<()>>;
}

const INPUT_PIN: u8 = 5;
const LED_PIN: u8 = 6;
const CHANNEL_SIZE: usize = 32;

pub async fn run(sig_rx: Receiver<()>) -> EResult<JoinHandle<()>> {
    let gpio = Gpio::new()?;
    let pin_led = gpio.get(LED_PIN)?;
    let pin_input = gpio.get(INPUT_PIN)?;

    let (sw_tx, sw_rx) = channel::bounded(CHANNEL_SIZE);

    let task_led = output::Output::new(sig_rx.clone(), sw_rx).run(pin_led)?; // LED
    let task_input = input::Input::new(sig_rx, sw_tx).run(pin_input)?; // 物理スイッチ

    let hdl = task::spawn(async move {
        // LEDタスク終了を待機
        task_led.await;

        // GPIO入力タスク終了を待機
        task_input.await;
    });

    println!("initialized GPIO");

    Ok(hdl)
}
