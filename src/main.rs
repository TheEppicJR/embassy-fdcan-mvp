#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use defmt::*;
use {defmt_rtt as _, panic_probe as _};
use embassy_executor::Spawner;
use embassy_stm32::can::{self, FdcanRx, NormalOperationMode};
use embassy_stm32::{bind_interrupts, gpio, Config, rcc};
use embassy_stm32::peripherals::*;
use embassy_time::Timer;
use embassy_stm32::time::Hertz;


bind_interrupts!(struct Irqs1 {
    FDCAN1_IT0 => can::IT0InterruptHandler<FDCAN1>;
    FDCAN1_IT1 => can::IT1InterruptHandler<FDCAN1>;
});

bind_interrupts!(struct Irqs2 {
    FDCAN2_IT0 => can::IT0InterruptHandler<FDCAN2>;
    FDCAN2_IT1 => can::IT1InterruptHandler<FDCAN2>;
});


#[embassy_executor::task]
async fn task1(mut rx: FdcanRx<'static, '_, FDCAN1, NormalOperationMode>) {
    info!("Task1");
    loop {
        // info!("FDCAN1: Waiting for frame");
        let received = rx.read().await;
        match received {
            Ok(frame) => {
                info!("FDCAN1: Received frame: {:?}", frame.data());
            },
            Err(e) => {
                info!("FDCAN1: Error: {:?}", e);
            }
        }
    }
}

#[embassy_executor::task]
async fn task2(mut tx: can::FdcanTx<'static, '_, FDCAN1, NormalOperationMode>) {
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
    
    loop {
        let send_frame = tx.write(&frame).await;
        match send_frame {
            Some(frame) => {
                info!("FDCAN1: Sent frame: {:?}", frame.data());
            },
            None => {
                info!("FDCAN1: Send Frame");
            }
        }
        Timer::after_millis(250).await;
    }
}

#[embassy_executor::task]
async fn task3(mut rx: FdcanRx<'static, '_, FDCAN2, NormalOperationMode>) {
    loop {
        let received = rx.read().await;
        match received {
            Ok(frame) => {
                info!("FDCAN2: Received frame: {:?}", frame.data());
            },
            Err(e) => {
                info!("FDCAN2: Error:: {:?}", e);
            }
        }
    }
}

#[embassy_executor::task]
async fn task4(mut tx: can::FdcanTx<'static, '_, FDCAN2, NormalOperationMode>) {
    let frame = can::TxFrame::new(
        can::TxFrameHeader {
            len: 8,
            frame_format: can::FrameFormat::Standard,
            id: can::StandardId::new(0x124).unwrap().into(),
            bit_rate_switching: false,
            marker: None,
        },
        &[8, 7, 6, 5, 4, 3, 2, 1]
    ).unwrap();
    
    loop {
        let sent_frame = tx.write(&frame).await;
        match sent_frame {
            Some(frame) => {
                info!("FDCAN2: Sent frame: {:?}", frame.data());
            },
            None => {
                info!("FDCAN2: Send Frame");
            }
        }
        Timer::after_millis(250).await;
    }
}

#[embassy_executor::main]
async fn main(spawner: Spawner) {

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

    let can1 = can::Fdcan::new(
        peripherals.FDCAN1,
        peripherals.PH14,
        peripherals.PH13,
        Irqs1
    );

    let can2 = can::Fdcan::new(
        peripherals.FDCAN2,
        peripherals.PB5,
        peripherals.PB13,
        Irqs2
    );

    can1.can.borrow_mut().apply_config(
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

    can2.can.borrow_mut().apply_config(
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

    let can1 = can1.into_normal_mode();
    let can2 = can2.into_normal_mode();
    
    let mut led = gpio::Output::new(peripherals.PA5, gpio::Level::High, gpio::Speed::Low);

    // make a static reference to the can peripheral so it cant get dropped
    static mut CAN1: Option<can::Fdcan<'static, FDCAN1, NormalOperationMode>> = None;
    static mut CAN2: Option<can::Fdcan<'static, FDCAN2, NormalOperationMode>> = None;
    unsafe {
        CAN1 = Some(can1);
        CAN2 = Some(can2);
        
    }

    let (tx1, rx1): (can::FdcanTx<'_, '_, FDCAN1, NormalOperationMode>, can::FdcanRx<'_, '_, FDCAN1, NormalOperationMode>);
    let (tx2, rx2): (can::FdcanTx<'_, '_, FDCAN2, NormalOperationMode>, can::FdcanRx<'_, '_, FDCAN2, NormalOperationMode>);

    unsafe {
        (tx1, rx1) = CAN1.as_mut().unwrap().split();
        (tx2, rx2) = CAN2.as_mut().unwrap().split();
    }
    
    info!("Configured");

    unwrap!(spawner.spawn(task2(tx1)));
    unwrap!(spawner.spawn(task4(tx2)));
    unwrap!(spawner.spawn(task1(rx1)));
    unwrap!(spawner.spawn(task3(rx2)));

    loop {
        led.toggle();
        Timer::after_millis(100).await;
    }
}
