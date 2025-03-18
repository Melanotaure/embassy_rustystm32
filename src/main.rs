#![no_std]
#![no_main]

use cortex_m::prelude::_embedded_hal_blocking_delay_DelayUs;
use defmt::info;
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_stm32::adc::{Adc, Temperature, VrefInt};
use embassy_stm32::exti::ExtiInput;
use embassy_stm32::gpio::Pull;
use embassy_stm32::peripherals::{ADC1, PC1};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;
use embassy_sync::signal::Signal;
use embassy_time::{Delay, Duration, Timer, WithTimeout};
use panic_probe as _;

enum ButtonState {
    PRESSED,
    IDLE,
}

static SIGNAL: Signal<ThreadModeRawMutex, ButtonState> = Signal::new();

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    info!("starting...");
    let p = embassy_stm32::init(Default::default());

    spawner.spawn(temp(p.PC1, p.ADC1)).unwrap();

    let b = ExtiInput::new(p.PA0, p.EXTI0, Pull::None);
    button(b).await;
}

async fn button(mut button: ExtiInput<'_>) {
    let mut button_state: ButtonState;

    loop {
        button.wait_for_high().await;
        info!("Button pressed");
        button_state = ButtonState::PRESSED;
        SIGNAL.signal(button_state);
        Timer::after_millis(200).await;
        button.wait_for_low().await;
        button_state = ButtonState::IDLE;
        SIGNAL.signal(button_state);
    }
}

#[embassy_executor::task]
async fn temp(_pin: PC1, adc: ADC1) {
    let mut delay = Delay;
    let mut adc = Adc::new(adc);
    // let mut pin = pin;
    let mut vrefint = adc.enable_vrefint();
    let mut temp = adc.enable_temperature();

    delay.delay_us(Temperature::start_time_us().max(VrefInt::start_time_us()));
    let vrefint_sample = adc.blocking_read(&mut vrefint);
    let convert_to_millivolts = |sample| {
        const VREFINT_MV: u32 = 1210;
        (u32::from(sample) * VREFINT_MV / u32::from(vrefint_sample)) as u16
    };
    let convert_to_celsius = |sample| {
        const V25: i32 = 760;
        const AVG_SLOPE: f32 = 2.5;

        let sample_mv = convert_to_millivolts(sample) as i32;
        (sample_mv - V25) as f32 / AVG_SLOPE + 25.0
    };

    info!("VrefInt: {}", vrefint_sample);
    const MAX_ADC_SAMPLE: u16 = (1 << 12) - 1;
    info!("VCCA: {}", convert_to_millivolts(MAX_ADC_SAMPLE));

    const INTERVAL_MS: u64 = 10;
    let mut delay_ms = INTERVAL_MS * 10;
    loop {
        // let v = adc.blocking_read(&mut pin);
        // info!("PC1: {} ({} mV)", v, convert_to_millivolts(v));

        let v = adc.blocking_read(&mut temp);
        let celcius = convert_to_celsius(v);
        info!("Internal temp: {} ({} °C)", v, celcius);

        // let v = adc.blocking_read(&mut vrefint);
        // info!("VrefInt: {}", v);

        let dly = Duration::from_millis(delay_ms);
        if let Some(v) = SIGNAL.wait().with_timeout(dly).await.ok() {
            delay_ms = match v {
                ButtonState::PRESSED if delay_ms > (INTERVAL_MS * 10) => {
                    delay_ms - INTERVAL_MS * 10
                }
                ButtonState::PRESSED => delay_ms,
                _ => delay_ms,
            };
            info!("Delay = {} ms", delay_ms);
        };
        delay_ms += INTERVAL_MS;
    }
}
