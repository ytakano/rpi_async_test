use rppal::gpio::Level;

pub enum Msg {
    GPIO(Level),
    Quit,
}
