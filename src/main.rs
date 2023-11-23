#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use {defmt_rtt as _, panic_probe as _};
use embassy_executor::Spawner;
use embassy_stm32::can;
use embassy_stm32::{bind_interrupts, gpio, Config, rcc};
use embassy_stm32::peripherals::*;
use embassy_time::Timer;

bind_interrupts!(struct Irqs {
    FDCAN1_IT0 => can::IT0InterruptHandler<FDCAN1>;
    FDCAN1_IT1 => can::IT1InterruptHandler<FDCAN1>;
});

#[embassy_executor::main]
async fn main(_spawner: Spawner) {

    let mut config = Config::default();

    // configure FDCAN to use PLL1_Q at 64 MHz
    config.rcc.pll1 = Some(rcc::Pll {
        source: rcc::PllSource::HSI,
        prediv: rcc::PllPreDiv::DIV4,
        mul: rcc::PllMul::MUL8,
        divp: None,
        divq: Some(rcc::PllDiv::DIV2),
        divr: None,
    });
    config.rcc.fdcan_clock_source = rcc::FdCanClockSource::PLL1_Q;

    let peripherals = embassy_stm32::init(config);

    let can = can::Fdcan::new(
        peripherals.FDCAN1,
        peripherals.PB12,
        peripherals.PB10,
        Irqs
    );

    can.can.borrow_mut().apply_config(
        can::config::FdCanConfig::default()
            .set_nominal_bit_timing(
                can::config::NominalBitTiming {
                    sync_jump_width: 1.try_into().unwrap(),
                    prescaler: 8.try_into().unwrap(),
                    seg1: 13.try_into().unwrap(),
                    seg2: 2.try_into().unwrap(),
                }
            )
    );

    let mut can = can.into_normal_mode();

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

    let mut led = gpio::Output::new(peripherals.PA5, gpio::Level::High, gpio::Speed::Low);

    loop {
        led.set_high();
        _ = can.write(&frame).await;
        Timer::after_millis(250).await;

        led.set_low();
        _ = can.write(&frame).await;
        Timer::after_millis(250).await;
    }
}
