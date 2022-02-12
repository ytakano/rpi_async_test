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
- [シグナル](./src/signal.rs)

## データベース

Dieselを利用して、PostgreSQLに保存します。
環境変数`DATABASE_URL`に適切な値を設定すると保存できます。

Deiselはasync/awaitで使うのが難しかったので、DB系は別スレッドで動作します。

```sh
$ export DATABASE_URL=postgres://user:pass@localhost/rpi_async
$ diesel migration run
$ ./target/release/rpi_async
```
