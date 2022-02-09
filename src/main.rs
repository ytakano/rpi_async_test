use async_std::{
    channel::{self, Receiver, Sender},
    task::{self, JoinHandle},
};
use rppal::gpio::{Gpio, InputPin, Level, OutputPin, Trigger};
use signal_hook::consts::signal::*;
use signal_hook_async_std::Signals;
use std::time::Duration;

#[allow(unused_imports)]
use async_std::prelude::*;

type RResult<T> = Result<T, Box<dyn std::error::Error>>;

const INPUT_PIN: u8 = 20;
const LED_PIN: u8 = 21;
const CHANNEL_SIZE: usize = 32;

macro_rules! perror {
    ($($args: expr),*) => {
        {
            print!("error: file: {}, line: {}", file!(), line!());
            $( print!(", {}: {}", stringify!($args), $args); )*
            println!(""); // to get a new line at the end
        }
    }
}

enum GPIOMsg {
    Input(Level),
    Quit,
}

#[async_std::main]
async fn main() -> RResult<()> {
    let gpio = Gpio::new()?;
    let pin_led = gpio.get(LED_PIN)?.into_output();
    let pin_sw = gpio.get(INPUT_PIN)?.into_input_pulldown();

    let (tx, rx) = channel::bounded(CHANNEL_SIZE);

    // GPIOの入力変化を送信
    let task_gpio = spawn_gpio_reader(pin_sw, tx.clone())?;

    // シグナルハンドラを起動
    let signals = Signals::new(&[SIGHUP, SIGTERM, SIGINT, SIGQUIT])?;
    spawn_signal_handler(signals, tx);

    switch_led(pin_led, rx).await;
    task_gpio.await;

    Ok(())
}

/// LEDをオン・オフ
async fn switch_led(mut pin: OutputPin, rx: Receiver<GPIOMsg>) {
    while let Ok(msg) = rx.recv().await {
        match msg {
            GPIOMsg::Input(level) => match level {
                Level::High => pin.set_high(),
                Level::Low => pin.set_low(),
            },
            GPIOMsg::Quit => {
                println!("quit");
                break;
            }
        }
    }
}

fn spawn_gpio_reader(mut pin: InputPin, tx: Sender<GPIOMsg>) -> RResult<JoinHandle<()>> {
    // GPIOの入力変化を送信
    pin.set_interrupt(Trigger::Both)?;
    let handler = task::spawn(async move {
        let t = Duration::from_millis(200);
        loop {
            match pin.poll_interrupt(false, Some(t)) {
                Ok(Some(level)) => {
                    println!("{level}");
                    if let Err(e) = tx.send(GPIOMsg::Input(level)).await {
                        perror!(e);
                        return;
                    }
                }
                Ok(None) => {
                    // timeout
                    let level = pin.read();
                    if let Err(_) = tx.send(GPIOMsg::Input(level)).await {
                        return;
                    }
                }
                Err(e) => {
                    perror!(e);
                    return;
                }
            }
        }
    });

    Ok(handler)
}

fn spawn_signal_handler(mut signals: Signals, tx: Sender<GPIOMsg>) {
    task::spawn(async move {
        while let Some(signal) = signals.next().await {
            match signal {
                SIGHUP => {
                    // Reload configuration
                    // Reopen the log file
                }
                SIGTERM | SIGINT | SIGQUIT => {
                    // Shutdown the system;
                    if let Err(e) = tx.send(GPIOMsg::Quit).await {
                        perror!(e);
                        return;
                    }
                }
                _ => unreachable!(),
            }
        }
    });
}
