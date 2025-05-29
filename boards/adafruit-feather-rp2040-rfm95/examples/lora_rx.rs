//! Receive packets using the LoRa 1276 module on the Adafruit Feather RP2040 RFM95 board
//!
//! This will blink the on-board LED when reception is successful. On failure and when polling the IRQ, the LED will remain on.
#![no_std]
#![no_main]

use adafruit_feather_rp2040_rfm95::{entry, hal};
use adafruit_feather_rp2040_rfm95::{
    hal::{
        clocks::init_clocks_and_plls,
        pac,
        watchdog::Watchdog,
        Sio,
    },
    Pins, XOSC_CRYSTAL_FREQ,
};
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::{OutputPin, PinState};
use fugit::RateExtU32;
use panic_halt as _;
use rp2040_hal::Clock;

// note: must match tx frequency in p2p config
// warning: set this appropriately for the region
const FREQUENCY: i64 = 915;

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    let mut watchdog = Watchdog::new(pac.WATCHDOG);

    let clocks = init_clocks_and_plls(
        XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
        .ok()
        .unwrap();

    let mut timer = rp2040_hal::Timer::new(pac.TIMER, &mut pac.RESETS, &clocks);

    let sio = Sio::new(pac.SIO);
    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let spi_mosi = pins.mosi.into_function::<hal::gpio::FunctionSpi>();
    let spi_miso = pins.miso.into_function::<hal::gpio::FunctionSpi>();
    let spi_sclk = pins.sclk.into_function::<hal::gpio::FunctionSpi>();
    let spi = hal::spi::Spi::<_, _, _, 8>::new(pac.SPI1, (spi_mosi, spi_miso, spi_sclk));
    let spi = spi.init(
        &mut pac.RESETS,
        clocks.peripheral_clock.freq(),
        16.MHz(),
        embedded_hal::spi::MODE_0,
    );

    let nss = pins.rfm_cs.into_push_pull_output_in_state(PinState::High);
    let reset = pins.rfm_rst.into_push_pull_output_in_state(PinState::High);
    let delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());
    let mut lora = sx127x_lora::LoRa::new(
        spi, nss, reset,  FREQUENCY, delay
    ).unwrap();

    let mut led = pins.led.into_push_pull_output();

    loop {
        match lora.poll_irq(Some(30)) { // 30 Second timeout
            Ok(_) => {
                for _ in 0..3 {
                    led.set_high().unwrap();
                    timer.delay_ms(500);
                    led.set_low().unwrap();
                    timer.delay_ms(500);
                }
            },
            Err(_) => {
                led.set_high().unwrap();
                timer.delay_ms(3000);
                led.set_low().unwrap();
            }
        }
    }
}