#![no_std]
#![no_main]

use core::fmt::Write;

// pick a panicking behavior
extern crate panic_halt; // you can put a breakpoint on `rust_begin_unwind` to catch panics

use cortex_m_rt::entry;

use embedded_hal as hal;
use hal::digital::v2::OutputPin;

use stm32g0xx_hal::{
    prelude::*,
    stm32,
    spi,
    serial::Config,
    gpio,
    exti::Event,
    rcc
};

use ssd1362::{self, display::DisplayRotation};

use embedded_graphics::{
    fonts::{Font6x8, Text, Font24x32},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Circle},
    style::{PrimitiveStyle, TextStyle, TextStyleBuilder},
};


#[entry]
fn main() -> ! {
    let dp = stm32::Peripherals::take().expect("cannot take peripherals");

    // let cfg = rcc::Config::pll();

    // default pll config is 64MHz

    // f_vco = 16 / 4 * 30 = 120
    // f = 120 / 2 = 60
    // let pll_cfg = rcc::PllConfig::with_hsi(4, 20, 2 );

    let pll_cfg = rcc::PllConfig::default();
    let rcc_cfg = rcc::Config::pll().pll_cfg(pll_cfg);
    let mut rcc = dp.RCC.freeze(rcc_cfg);

    // let mut rcc = dp.RCC.constrain();

    let mut delay = dp.TIM15.delay(&mut rcc);

    let gpioa = dp.GPIOA.split(&mut rcc);
    let gpiob = dp.GPIOB.split(&mut rcc);
    let gpioc = dp.GPIOC.split(&mut rcc);

    let btn = gpioc.pc13.into_pull_up_input();
    let mut exti = dp.EXTI;
    btn.listen(gpio::SignalEdge::Falling, &mut exti);

    let mut led = gpioa.pa5.into_push_pull_output();


    let mut cs = gpiob.pb0.into_push_pull_output(); // blue 13
    cs.set_high().unwrap();

    // d/c pin for toggling between data/cmd
    // dc low = command, dc high = data
    let mut dc = gpioa.pa7.into_push_pull_output(); // orange
    dc.set_low().unwrap();

    let mut rst = gpioa.pa6.into_push_pull_output();
    rst.set_high().unwrap();


    let mut usart = dp
    .USART2 // tx      // rx
    .usart(gpioa.pa2, gpioa.pa3, Config::default().baudrate(1000000.bps()), &mut rcc)
    .unwrap();

    writeln!(usart, "Hello stm32g0\n").unwrap();


    led.set_high().unwrap();
    delay.delay(500.ms());
    led.set_low().unwrap();

    let sck = gpiob.pb3; // yellow 10
    let miso = gpiob.pb4;
    let mosi = gpiob.pb5; // green 9
    let spi = dp.SPI1.spi(
        (sck, miso, mosi),
        spi::MODE_0,
        10.mhz(),
        &mut rcc);


    writeln!(usart, "Start!").unwrap();

    // do power on reset
    rst.set_low().unwrap();
    delay.delay(1.ms());
    rst.set_high().unwrap();

    // send configuration bytes
    cs.set_low().unwrap();

    writeln!(usart, "Turn on VCC!").unwrap();

    let spi_interface = ssd1362::interface::SpiInterface::new(spi, cs, dc);
    let mut display = ssd1362::display::Display::new(spi_interface, DisplayRotation::Rotate0);
    display.init().unwrap();
    display.clear(BinaryColor::Off);
    display.on().unwrap();
    display.flush().unwrap();


    display.write_char('@', 0,0).unwrap();


    display.write_char('=', 1,1).unwrap();

    display.write_char('?', 2,2).unwrap();


    display.write_string("Hello, Jitter!", 4, 4).unwrap();

    delay.delay(2000.ms());

    let c = Circle::new(Point::new(20, 20), 12).into_styled(PrimitiveStyle::with_fill(BinaryColor::On));
    let t = Text::new("Hello Rust!", Point::new(120, 16))
        .into_styled(TextStyle::new(Font6x8, BinaryColor::On));

    c.draw(&mut display);
    t.draw(&mut display);



    let style = TextStyleBuilder::new(Font24x32).background_color(BinaryColor::On).text_color(BinaryColor::Off).build();
    Text::new("YES!", Point::new(120, 30))
    .into_styled(style).draw(&mut display);


    display.flush().unwrap();


    let mut scroll_offset = 0u8;

    loop {

        if exti.is_pending(Event::GPIO13, gpio::SignalEdge::Falling) {
            led.toggle().unwrap();
            exti.unpend(Event::GPIO13);


            writeln!(usart, "Werkt nog").unwrap();

            display.draw(&[0x0F; 1]).unwrap();
            display.draw(&[0xF0; 1]).unwrap();


        }


        delay.delay(50.ms());

        display.scroll(scroll_offset).unwrap();
        scroll_offset += 8;

        if scroll_offset > 63 {
            scroll_offset = 0;
        }

    }
}
