# Raspberry Pi 4 in Rust

Raspberry Pi 4をRustのasync/awaitの非同期プログラミングで操作するテストコードです。
async_stdとrppalを用いています。

![Raspberry Pi 4](./materials/rpi4.jpeg)

シグナルを受け取ると、Graceful shutdownします。

- GPIO
  - [入力](./src/gpio/input.rs)
  - [出力](./src/gpio/output.rs)
- I2C
  - [ADT7410、温度センサ](./src/i2c/adt7410.rs)
  - [ST7032、ディスプレイ](./src/i2c/st7032.rs)
  - [CCS811、二酸化炭素・総揮発性有機化合物](./src/i2c/ccs811.rs)
- SPI
  - [MCP3208、ADコンバータ](./src/spi/mcp3208.rs)
- [シグナル](./src/signal.rs)

## データベース

Dieselを利用して、PostgreSQLに保存します。
環境変数`DATABASE_URL`に適切な値を設定すると保存できます。

Deiselはasync/awaitで使うのが難しかったので、DB系は別スレッドで動作します。

```sh
$ export DATABASE_URL=postgres://user:pass@localhost/rpi_async
$ diesel setup
$ ./target/release/rpi_async
```

DBに保存したデータは以下のようにGrafana等で可視化できます。

![Grafana](./materials/grafana.png)

## 参考資料

CCS811以外の使い方は、以下の書籍に載っています。実装はPythonですが、とても良い本です。

[Raspberry Piで学ぶ電子工作](https://amzn.to/3p7agcn)

## 配線

ピン配置

```
   3V3  (1) (2)  5V
 GPIO2  (3) (4)  5V
 GPIO3  (5) (6)  GND
 GPIO4  (7) (8)  GPIO14
   GND  (9) (10) GPIO15
GPIO17 (11) (12) GPIO18
GPIO27 (13) (14) GND
GPIO22 (15) (16) GPIO23
   3V3 (17) (18) GPIO24
GPIO10 (19) (20) GND
 GPIO9 (21) (22) GPIO25
GPIO11 (23) (24) GPIO8
   GND (25) (26) GPIO7
 GPIO0 (27) (28) GPIO1
 GPIO5 (29) (30) GND
 GPIO6 (31) (32) GPIO12
GPIO13 (33) (34) GND
GPIO19 (35) (36) GPIO16
GPIO26 (37) (38) GPIO20
   GND (39) (40) GPIO21
```

- GPIO11 (23)はSPI SCLK
- GPIO09 (21)はSPI MISO
- GPIO10 (10)はSPI MOSI
- GPIO08 (24)はSPI CEO
- GPIO02 (3)はI2C SDA
- GPIO03 (5)はI2C SCL

### MCP3208 (SPI)

```
CH0 Vdd     -> 3V3
CH1 Vref    -> 3V3
CH2 AGND    -> GND
CH3 CLK     -> GPIO11 (23), SPI SCLK
CH4 Dout    -> GPIO01 (21), SPI MISO
CH5 Din     -> GPIO10 (19), SPI MOSI
CH6 CS/SHDN -> GPIO08 (24), SPI CEO
CH7 DGND    -> GND
```

フォトレジスタの接続例は以下。

```
3V3 - CdS Cell - 10 KΩ抵抗 - GND
               |
              CH0
```

### ADT7410, ST7032 (I2C)

```
Vdd -> 3V3
SCL -> GPIO03 (5), I2C SCL
SDA -> GPIO02 (3), I2C SDA
GND -> GND
```

### CSS811 (I2C)

```
Vdd -> 3V3
SCL -> GPIO03 (5), I2C SCL
SDA -> GPIO02 (3), I2C SDA
GND -> GND
WAK -> (40) GPIO21
RST -> 3V3
ADD -> GND
```

GPIO21は他のGPIOピンでも問題なし。

ADDはGNDか3V3かによって、I2Cのアドレスが以下のように変化する。

- ADD
  - GND -> 0x5a
  - 3V3 -> 0x5b

### GPIO

GPIOの入力と出力のテスト。
入力のタクトスイッチを押すと、LEDが点灯。

#### 出力

```
GPIO06 (31) - LED - 330 Ω抵抗 - GND
```

#### 入力

```
3V3 - タクトスイッチ - GPIO05 (29)
```
