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

pub use display_interface_spi::{SPIInterface, SPIInterfaceNoCS};
use ssd1362::{self, display::DisplayRotation, terminal};


#[entry]
fn main() -> ! {

    let dp = stm32::Peripherals::take().expect("cannot take peripherals");

    let pll_cfg = rcc::PllConfig::default();
    let rcc_cfg = rcc::Config::pll().pll_cfg(pll_cfg);
    let mut rcc = dp.RCC.freeze(rcc_cfg);

    let mut delay = dp.TIM15.delay(&mut rcc);

    let gpioa = dp.GPIOA.split(&mut rcc);
    let gpiob = dp.GPIOB.split(&mut rcc);
    let _gpioc = dp.GPIOC.split(&mut rcc);

    let btn = gpioa.pa8.into_pull_up_input();
    let mut exti = dp.EXTI;
    btn.listen(gpio::SignalEdge::Falling, &mut exti);

    // let mut led = gpioa.pa5.into_push_pull_output();
    let mut led_g = gpioa.pa7.into_push_pull_output();
    let mut led_r = gpiob.pb0.into_push_pull_output();
    let mut en_16v = gpioa.pa1.into_push_pull_output();


    let mut cs = gpiob.pb4.into_push_pull_output(); // blue 13
    cs.set_high().unwrap();

    // d/c pin for toggling between data/cmd
    // dc low = command, dc high = data
    let mut dc = gpiob.pb7.into_push_pull_output(); // orange
    dc.set_low().unwrap();

    let mut rst = gpiob.pb6.into_push_pull_output();
    rst.set_high().unwrap();

    let mut usart = dp
    .USART1 // tx      // rx
    .usart(gpioa.pa9, gpioa.pa10, Config::default().baudrate(115200.bps()), &mut rcc)
    .unwrap();

    let mut usart3 = dp
    .USART3 // tx      // rx
    .usart(gpiob.pb8, gpiob.pb9, Config::default().baudrate(115200.bps()), &mut rcc)
    .unwrap();

    writeln!(usart3, "Hello stm32g0\n").unwrap();

    led_g.set_high().unwrap();
    delay.delay(500.ms());
    led_g.set_low().unwrap();

    let sck = gpiob.pb3; // yellow 10
    let miso = gpioa.pa6; //not used
    let mosi = gpiob.pb5; // green 9
    let spi = dp.SPI1.spi(
        (sck, miso, mosi),
        spi::MODE_0,
        10.mhz(),
        &mut rcc);


    // do power on reset
    rst.set_low().unwrap();
    delay.delay(1.ms());
    rst.set_high().unwrap();


    writeln!(usart, "Turn on VCC!").unwrap();
    en_16v.set_high().unwrap();

    let interface = display_interface_spi::SPIInterface::new(spi, dc, cs);
    let display = ssd1362::display::Display::new(interface, DisplayRotation::Rotate180);
    writeln!(usart, "create terminal..").unwrap();
    let font = terminal::Font6x8 {};
    let mut terminal = terminal::TerminalView::new(display, font);
    terminal.init().unwrap();



    writeln!(usart, "Display init done!").unwrap();

    // write!(terminal, "=> regel 1.\n\n").unwrap();
    // writeln!(terminal, "").unwrap();
    // terminal.write_string("\n").unwrap();
    // write!(terminal, " ").unwrap();
    writeln!(terminal, "AA").unwrap();
    terminal.render();

    writeln!(terminal, "BB").unwrap();
    terminal.render();

    writeln!(terminal, "CC").unwrap();
    terminal.render();
    writeln!(terminal, "0 Lange zin van twee regels la").unwrap();
    terminal.render();
    writeln!(terminal, "1").unwrap();
    writeln!(terminal, "2").unwrap();
    writeln!(terminal, "1").unwrap();
    writeln!(terminal, "2").unwrap();
    writeln!(terminal, "1").unwrap();
    writeln!(terminal, "2").unwrap();
    writeln!(terminal, "3").unwrap();
    terminal.render();

    writeln!(terminal, "0 sunt aut. Molestiae est nihi").unwrap();
    writeln!(terminal, "2 Nam eligendi dolore hic.").unwrap();

    writeln!(terminal, "1 Eum sunt qui est id officiis N").unwrap();
    write!(terminal, "2 Nam eligendi dolore hic.").unwrap();

    writeln!(terminal, "1 Eum sunt qui est id officiis Nam eligendi dolore hic. Est ratione pariatur dolores. repudiandae aut iure ipsum.").unwrap();
    writeln!(terminal, "1 Eum sunt qui ").unwrap();
    terminal.render();

    writeln!(terminal, "1 Eum sunt qui est id offic").unwrap();
    terminal.render();

    writeln!(terminal, "2 Nam eligendi dolore hic.").unwrap();
    terminal.render();
    writeln!(terminal, "3 Est ratione pariatur dolores").unwrap();
    terminal.render();
    writeln!(terminal, "4 repudiandae aut iure ipsum").unwrap();
    terminal.render();
    writeln!(terminal, "5 molestias. Enim quis dolorem").unwrap();
    terminal.render();
    writeln!(terminal, "6 Beatae eligendi eius et cons").unwrap();
    terminal.render();
    writeln!(terminal, "7 sunt aut. Molestiae est nihi").unwrap();
    terminal.render();
    writeln!(terminal, "8 laborum eligendi inventore.").unwrap();
    terminal.render();
    writeln!(terminal, "9 Lorem ipsum dolor sit amet").unwrap();
    terminal.render();
    writeln!(terminal, "10 consectetur adipiscing elit").unwrap();
    terminal.render();
    writeln!(terminal, "11 sed do eiusmod tempor sed").unwrap();
    terminal.render();
    writeln!(terminal, "12 do eiusmod tempor incididunt").unwrap();
    terminal.render();
    writeln!(terminal, "13 sed ut labore et dolore magna").unwrap();
    terminal.render();
    writeln!(terminal, "14 aliqua. Ut enim ad minim veniam").unwrap();
    terminal.render();
    writeln!(terminal, "15 quis nostrud exercitation ullamco").unwrap();

    terminal.render();

    writeln!(terminal, "AA").unwrap();
    terminal.render();

    writeln!(terminal, "BB").unwrap();
    terminal.render();

    writeln!(terminal, "CC").unwrap();
    terminal.render();

    loop {
        delay.delay(1500.ms());
        led_g.toggle().unwrap();
        led_r.toggle().unwrap();
    }

    let mut scroll: i32 = 0;
    let mut step: i32 = 1;
    loop {
        terminal.set_scroll_offset(scroll as usize);
        terminal.render();

        scroll += step;
        if scroll % 8 == 0 {
            step *= -1;
        }

        if exti.is_pending(Event::GPIO8, gpio::SignalEdge::Falling) {
            led_r.toggle().unwrap();
            exti.unpend(Event::GPIO8);
            writeln!(usart, "info: Still working").unwrap();
            if step == 0 {
                step = 1;
            } else {
                step = 0;
            }
        }
        delay.delay(50.ms());
    }
}
