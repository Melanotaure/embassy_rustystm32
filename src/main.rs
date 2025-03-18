#![no_std]
#![no_main]

use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::Pull;
use embassy_time::Timer;
use panic_probe as _;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    info!("starting...");
    let p = embassy_stm32::init(Default::default());
    let mut button = ExtiInput::new(p.PA0, p.EXTI0, Pull::None);
    loop {
        button.wait_for_high().await;
        info!("Button pressed");
        Timer::after_millis(200).await;
        button.wait_for_low().await;
    }
}
