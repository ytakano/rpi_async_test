# Raspi4 in Rust

Raspi4をRustのasync/awaitの非同期プログラミングで操作するテストコードです。
async_stdとrppalを用いています。

シグナルを受け取ると、Graceful shutdownします。

- GPIO
  - [入力](./src/gpio/input.rs)
  - [出力](./src/gpio/output.rs)
- I2C
  - [ADT7410、温度センサ](./src/i2c/adt7410.rs)
  - [ST7032、ディスプレイ](./src/i2c/st7032.rs)
- SPI
  - [MCP3208、ADコンバータ](./src/spi/mcp3208.rs)
- シグナル
  - [シグナルハンドラ](./src/signal.rs)

![Rpi4](./materials/rpi4.jpeg)
