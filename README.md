# SSD1362 OLED Driver crate

SPI Driver crate for SSD1362 OLED displays with resolution of 256x64.

How to build and run:

```
cargo build --example terminal
cargo run --example terminal
```

Example usage:

```rust
use display_interface_spi::{SPIInterface, SPIInterfaceNoCS};
use ssd1362::{self, display::DisplayRotation, terminal};

// ... code omitted

let spi = dp.SPI1.spi(
    (sck, miso, mosi),
    spi::MODE_0,
    10.mhz(),
    &mut rcc);

// do power on reset
rst.set_low().unwrap();
delay.delay(1.ms());
rst.set_high().unwrap();

en_16v.set_high().unwrap();

let interface = display_interface_spi::SPIInterface::new(spi, dc, cs);
let display = ssd1362::display::Display::new(interface, DisplayRotation::Rotate180);
let font = terminal::Font6x8 {};
let mut terminal = terminal::TerminalView::new(display, font);
terminal.init().unwrap();

writeln!(terminal, "Write a string to the terminal").unwrap();
terminal.render();
```