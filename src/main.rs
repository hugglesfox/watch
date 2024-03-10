#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]

use defmt_rtt as _; // global logger 
use panic_probe as _; // panic handler

mod button;
mod buzzer;
mod measurement;
mod rtc;

#[rtic::app(
    device = stm32l0xx_hal::pac,
    dispatchers = [I2C1, I2C2, I2C3]
)]
mod app {
    use crate::measurement::{Temperature, Voltage};
    use crate::buzzer::Buzzer;

    use cortex_m::peripheral::SCB;
    use stm32l0xx_hal::prelude::*;

    use stm32l0xx_hal::adc::{Adc, VRef, VTemp};
    use stm32l0xx_hal::exti::{Exti, GpioLine};
    use stm32l0xx_hal::pwm::Timer;
    use stm32l0xx_hal::pwr::PWR;
    use stm32l0xx_hal::rcc::Rcc;
    use stm32l0xx_hal::rtc::Rtc;
    use stm32l0xx_hal::syscfg::SYSCFG;
    use stm32l0xx_hal::{adc, pwr, rcc, rtc};

    use rtic_monotonics::systick::{fugit::ExtU32, Systick};


    #[shared]
    struct Shared {
        // Peripherals
        pwr: PWR,
        rcc: Rcc,
        rtc: Rtc,
        scb: SCB,

        // State
        /// Whether buzzer is allowed to sound (toggable with the alarm button)
        #[lock_free]
        buzzer_enabled: bool,
    }

    #[local]
    struct Local {
        adc: Adc<adc::Ready>,
        buzzer: Buzzer,
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        defmt::info!("init");

        let dp = cx.device;
        let cp = cx.core;

        // Set clock to MSI range 0
        let mut rcc = dp.RCC.freeze(rcc::Config::msi(rcc::MSIRange::Range0));
        let mut pwr = PWR::new(dp.PWR, &mut rcc);

        pwr.enter_low_power_run_mode(rcc.clocks);

        let mut exti = Exti::new(dp.EXTI);
        let mut syscfg = SYSCFG::new(dp.SYSCFG, &mut rcc);

        let adc = Adc::new(dp.ADC, &mut rcc);

        let gpioa = dp.GPIOA.split(&mut rcc);
        let gpiob = dp.GPIOB.split(&mut rcc);

        // Setup buzzer
        let timer = Timer::new(dp.TIM2, 440.Hz(), &mut rcc);
        let buzzer = timer.channel1.assign(gpioa.pa0);

        // Setup systick timer
        let systick_token = rtic_monotonics::create_systick_token!();
        Systick::start(cp.SYST, rcc.clocks.sys_clk().0, systick_token);

        // Setup buttons
        let alarm_btn = gpioa.pa2.into_pull_down_input();
        let mode_btn = gpiob.pb9.into_pull_down_input();

        use crate::button::Button as _;
        alarm_btn.enable_interrupt(&mut exti, &mut syscfg);
        mode_btn.enable_interrupt(&mut exti, &mut syscfg);

        // Setup RTC
        let mut rtc = Rtc::new(dp.RTC, &mut rcc, &pwr, rtc::ClockSource::LSE, None).unwrap();

        use crate::rtc::RtcInt as _;
        rtc.enable_wakeup_interrupt(&mut exti);

        // Start wakeup timer to update watch face every second
        rtc.wakeup_timer().start(1_u32);

        // Start calibration task
        calibrate::spawn().unwrap();        

        (
            Shared {
                pwr,
                rcc,
                rtc,
                scb: cp.SCB,
                buzzer_enabled: true,
            },
            Local { adc, buzzer },
        )
    }

    #[idle(shared = [pwr, scb, rcc])]
    fn idle(cx: idle::Context) -> ! {
        defmt::info!("idle");

        let pwr = cx.shared.pwr;
        let scb = cx.shared.scb;
        let rcc = cx.shared.rcc;

        (pwr, scb, rcc).lock(|pwr, mut scb, mut rcc| {
            defmt::info!("entering stop mode");
            // ULP needs to be disabled for the LCD to work
            pwr.stop_mode(
                &mut scb,
                &mut rcc,
                pwr::StopModeConfig {
                    ultra_low_power: false,
                },
            )
            .enter();
        });

        // Should never be reached
        loop {}
    }

    #[task(binds = RTC, shared = [rtc, buzzer_enabled])]
    fn wakeup(cx: wakeup::Context) {
        defmt::info!("rtc wakeup");

        let mut time = rtc::NaiveDateTime::default();
        let mut rtc = cx.shared.rtc;

        rtc.lock(|rtc| {
            // Clear interrupt
            rtc.wakeup_timer().wait().unwrap();
            time = rtc.now();
        });

        use stm32l0xx_hal::rtc::Timelike as _;

        // Sound the buzzer on the top of the hour if enabled
        if *cx.shared.buzzer_enabled && time.minute() == 0 && time.second() == 0 {
            if let Err(_) = beep::spawn() {
                defmt::error!("unable to spawn beep, already running");
            }
        }
    }

    /// Toggle `buzzer_enabled` every time the alarm button is pressed
    #[task(binds = EXTI2_3, shared = [buzzer_enabled])]
    fn alarm_btn(cx: alarm_btn::Context) {
        defmt::info!("alarm button");

        // Clear the interrupt (alarm button is on PA2)
        use stm32l0xx_hal::exti::ExtiLine as _;
        Exti::unpend(GpioLine::from_raw_line(2).unwrap());

        let buzzer_enabled = cx.shared.buzzer_enabled;

        *buzzer_enabled = !*buzzer_enabled;
    }

    #[task(priority = 2, local = [buzzer])]
    async fn beep(cx: beep::Context) {
        defmt::info!("beep");

        let buzzer = cx.local.buzzer;

        buzzer.enable();
        Systick::delay(1.secs()).await;
        buzzer.disable();
    }

    #[task(priority = 2, local = [adc], shared = [rtc])]
    async fn calibrate(cx: calibrate::Context) {
        defmt::info!("calibrate rtc");
        let adc = cx.local.adc;

        let mut vtemp = VTemp::new();

        loop {
            defmt::info!("starting an rtc calibration");
            vtemp.enable(adc);
            adc.calibrate().unwrap();

            let temp: Temperature = adc.read(&mut vtemp).unwrap();
            defmt::info!("Temperature {}°C", *temp);

            vtemp.disable(adc);

            // XXX: Implement setting the rtc calibration values in the hal

            Systick::delay(ExtU32::minutes(15)).await;
        }
    }
}
