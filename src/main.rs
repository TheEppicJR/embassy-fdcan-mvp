#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use defmt::*;
use {defmt_rtt as _, panic_probe as _};
use embassy_executor::Spawner;
use embassy_stm32::can;
use embassy_stm32::{bind_interrupts, gpio, Config, rcc};
use embassy_stm32::peripherals::*;
use embassy_time::Timer;
use embassy_stm32::time::Hertz;


bind_interrupts!(struct Irqs {
    FDCAN1_IT0 => can::IT0InterruptHandler<FDCAN1>;
    FDCAN1_IT1 => can::IT1InterruptHandler<FDCAN1>;
});


#[embassy_executor::main]
async fn main(_spawner: Spawner) {

    let mut config = Config::default();

    // configure FDCAN to use PLL1_Q at 64 MHz
    config.rcc.hse = Some(rcc::Hse{freq: Hertz(25_000_000), mode: rcc::HseMode::Oscillator});
    config.rcc.pll1 = Some(rcc::Pll {
        source: rcc::PllSource::HSE,
        prediv: rcc::PllPreDiv::DIV2,
        mul: rcc::PllMul::MUL12,
        divp: Some(rcc::PllDiv::DIV2),
        divq: Some(rcc::PllDiv::DIV3),
        divr: Some(rcc::PllDiv::DIV2),
    });
    config.rcc.fdcan_clock_source = rcc::FdCanClockSource::PLL1_Q;

    let peripherals = embassy_stm32::init(config);

    let can = can::Fdcan::new(
        peripherals.FDCAN1,
        peripherals.PH14,
        peripherals.PH13,
        Irqs
    );

    can.can.borrow_mut().apply_config(
        can::config::FdCanConfig::default()
            .set_nominal_bit_timing(
                can::config::NominalBitTiming {
                    sync_jump_width: 10.try_into().unwrap(),
                    prescaler: 10.try_into().unwrap(),
                    seg1: 2.try_into().unwrap(),
                    seg2: 2.try_into().unwrap(),
                }
            )
    );
    info!("1");
    let mut can = can.into_external_loopback_mode();
    info!("2");
    let frame = can::TxFrame::new(
        can::TxFrameHeader {
            len: 8,
            frame_format: can::FrameFormat::Standard,
            id: can::StandardId::new(0x123).unwrap().into(),
            bit_rate_switching: false,
            marker: None,
        },
        &[1, 2, 3, 4, 5, 6, 7, 8]
    ).unwrap();
    info!("3");
    let mut led = gpio::Output::new(peripherals.PA5, gpio::Level::High, gpio::Speed::Low);

    info!("Configured");

    loop {
        // led.set_high();
        _ = can.write(&frame).await;
        info!("sent");
        let received = can.read().await;
        info!("received");
        match received {
            Ok(frame) => {
                info!("Received frame: {:?}", frame.data());
            },
            Err(e) => {
                info!("Error: ");
            }
        }
        Timer::after_millis(250).await;

        // led.set_low();
        // _ = can.write(&frame).await;
        
        // Timer::after_millis(25).await;
    }
}
