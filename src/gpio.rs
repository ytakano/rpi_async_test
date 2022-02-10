#[allow(unused_imports)]
use async_std::prelude::*;

use super::{perror, EResult};
use async_std::{
    channel::{self, Receiver, Sender},
    task::{self, JoinHandle},
};
use rppal::gpio::{Gpio, InputPin, Level, OutputPin, Trigger};
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use std::time::Duration;

const INPUT_PIN: u8 = 5;
const LED_PIN: u8 = 6;
const CHANNEL_SIZE: usize = 32;

enum Msg {
    GPIO(Level),
    Quit,
}

pub async fn run(halt: Arc<AtomicBool>) -> EResult<JoinHandle<()>> {
    let gpio = Gpio::new()?;
    let pin_led = gpio.get(LED_PIN)?.into_output();
    let mut pin_sw = gpio.get(INPUT_PIN)?.into_input_pulldown();
    pin_sw.set_interrupt(Trigger::Both)?;

    let (tx, rx) = channel::bounded(CHANNEL_SIZE);

    // GPIOの入力変化を送信
    let reader_hdl = task::spawn(gpio_reader(pin_sw, tx, halt));

    let hdl = task::spawn(async move {
        // LEDタスクを実行
        switch_led(pin_led, rx).await;

        // LEDタスクが終了すると、GPIOタスクは必ず終了しているはず
        reader_hdl.await;
    });

    println!("initialized GPIO");

    Ok(hdl)
}

/// LEDをオン・オフ
async fn switch_led(mut pin: OutputPin, rx: Receiver<Msg>) {
    while let Ok(msg) = rx.recv().await {
        match msg {
            Msg::GPIO(level) => match level {
                Level::High => pin.set_high(),
                Level::Low => pin.set_low(),
            },
            Msg::Quit => {
                println!("exiting switch_led ...");
                break;
            }
        }
    }
}

async fn gpio_reader(mut pin: InputPin, tx: Sender<Msg>, halt: Arc<AtomicBool>) {
    // GPIOの入力変化を送信
    let t = Duration::from_millis(200);
    loop {
        match pin.poll_interrupt(false, Some(t)) {
            Ok(Some(level)) => {
                let p = pin.pin();
                println!("GPIO({p}): {level}");
                if let Err(_) = tx.send(Msg::GPIO(level)).await {
                    println!("exiting gpio_reader ...");
                    break;
                }
            }
            Ok(None) => {
                // timeout
                if halt.load(Ordering::Relaxed) {
                    let _ = tx.send(Msg::Quit).await;
                    break;
                }

                // 現在のレベルを送信
                let level = pin.read();
                if let Err(e) = tx.send(Msg::GPIO(level)).await {
                    perror!(e);
                    println!("exiting gpio_reader ...");
                    break;
                }
            }
            Err(e) => {
                perror!(e);
                break;
            }
        }
    }
}
