#![no_std]
#![no_main]

use cortex_m::delay::Delay as CortexDelay;
use cortex_m_rt::entry;
use embedded_time::rate::*;
use rp_pico::{
    hal::{
        clocks,
        gpio::{FunctionSpi, Pin},
        pac::{CorePeripherals, Peripherals},
        prelude::*,
        Sio, Spi, Watchdog,
    },
    Pins,
};

use embedded_hal::spi;

use panic_halt as _;

use embedded_hal::adc::OneShot;
use embedded_hal::PwmPin;

use embedded_hal::digital::v2::OutputPin;

mod screen;
use screen::Sh1106;

use chip8::pal::*;
mod keypad;

use keypad::GpioKeypad;

pub mod types;

// impl Delay for CortexDelay {
//     fn delay_us(&mut self, us: u32) -> Result<(), Self::Error> {
//         self.delay_us(us);
//         Ok(())
//     }
// }

#[entry]
fn main() -> ! {
    let mut pac = Peripherals::take().unwrap();
    let core = CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);

    let clocks = clocks::init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let mut delay = Delay::new(core.SYST, clocks.system_clock.freq().integer());

    let sio = Sio::new(pac.SIO);
    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut screen = {
        let spi: Spi<_, _, 8> = Spi::new(pac.SPI1).init(
            &mut pac.RESETS,
            clocks.peripheral_clock.freq(),
            30_000_000u32.Hz(),
            &spi::MODE_0,
        );

        let cs = pins.gpio9.into_push_pull_output();
        let dcmd = pins.gpio8.into_push_pull_output();
        let reset = pins.gpio12.into_push_pull_output();

        let _: Pin<_, FunctionSpi> = pins.gpio10.into_mode();
        let _: Pin<_, FunctionSpi> = pins.gpio11.into_mode();

        let mut screen = Sh1106::new(spi, cs, dcmd, reset);
        screen.init().ok().unwrap();
        screen
    };

    let mut keypad = {
        let mut keypad = GpioKeypad::new(
            pins.gpio0.into_push_pull_output(),
            pins.gpio1.into_push_pull_output(),
            pins.gpio2.into_push_pull_output(),
            pins.gpio3.into_push_pull_output(),
            pins.gpio4.into_pull_down_input(),
            pins.gpio5.into_pull_down_input(),
            pins.gpio13.into_pull_down_input(),
            pins.gpio14.into_pull_down_input(),
        );

        keypad.init().ok().unwrap();
        keypad
    };

    let sq = [1, 2, 4, 8, 16, 32, 64, 128];

    screen.xor(2, 0, &sq);
    screen.xor(48, 23, &sq);

    let mut led = pins.led.into_push_pull_output();
    led.set_high().ok();

    // let ram = chip8::ram::Ram::new();

    // let sprite = |key: u8| -> &[u8] {
    //     let sprite = ram.get_sprite_addr(key);
    //     ram.read_bytes(sprite, 5)
    // };

    // loop {
    //     match keypad.read_key(&mut delay) {
    //         Ok(Some(key)) => {
    //             screen.clear();
    //             screen.xor(2, 8, sprite(key));
    //             led.set_low().ok();
    //         }
    //         Ok(None) => {
    //             led.set_high().ok();
    //         }
    //         _ => {
    //             led.set_high().ok();
    //         }
    //     }

    //     delay.delay_ms(100);
    // }

    loop {}
}
