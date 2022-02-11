#[allow(unused_imports)]
use async_std::prelude::*;

use super::EResult;
use async_std::{channel::Receiver, task::JoinHandle};
use std::sync::{atomic::AtomicU64, Arc};

mod mcp3208;

trait Runner {
    fn run(self) -> EResult<JoinHandle<()>>;
}

pub async fn run(sig_rx: Receiver<()>, bright: Arc<AtomicU64>) -> EResult<JoinHandle<()>> {
    let hdl = mcp3208::MCP3208::new(sig_rx, bright).run()?;
    println!("initialized SPI");
    Ok(hdl)
}
