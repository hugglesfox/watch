#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]

use defmt_rtt as _; // global logger
use panic_probe as _; // panic handler

mod buzzer;
mod buttons;
mod measurement;

#[rtic::app(
    device = stm32l0xx_hal::pac,
    dispatchers = [I2C1, I2C2, I2C3]
)]
mod app {
    use crate::buzzer::{self, Buzzer};
    use crate::buttons;
    use crate::measurement::{Temperature, Voltage};

    use cortex_m::peripheral::SCB;
    use stm32l0xx_hal::prelude::*;

    use stm32l0xx_hal::exti::{Exti, GpioLine};
    use stm32l0xx_hal::pwr::PWR;
    use stm32l0xx_hal::rcc::Rcc;
    use stm32l0xx_hal::rtc::Rtc;
    use stm32l0xx_hal::syscfg::SYSCFG;
    use stm32l0xx_hal::adc::{Adc, VTemp, VRef};
    use stm32l0xx_hal::{adc, exti, pwr, rcc, rtc};

    use rtic_monotonics::systick::{fugit::ExtU32, Systick};

    #[shared]
    struct Shared {
        // Peripherals
        pwr: PWR,
        scb: SCB,
        rcc: Rcc,

        // State

        /// Whether buzzer is allowed to sound (toggable with the alarm button)
        #[lock_free]
        buzzer_enabled: bool,
        temperature: Temperature,
        voltage: Voltage,
    }

    #[local]
    struct Local {
        buzzer: Buzzer,
        rtc: Rtc,
        adc: Adc<adc::Ready>,
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
        let buzzer = buzzer::init(dp.TIM2, gpioa.pa0, 440.Hz(), &mut rcc);

        // Setup systick timer
        let systick_token = rtic_monotonics::create_systick_token!();
        Systick::start(cp.SYST, rcc.clocks.sys_clk().0, systick_token);


        // Setup buttons
        let alarm_btn = gpioa.pa2.into_pull_down_input();
        let mode_btn = gpiob.pb9.into_pull_down_input();
        buttons::init(alarm_btn, mode_btn, &mut exti, &mut syscfg);

        // Setup RTC
        let mut rtc = Rtc::new(dp.RTC, &mut rcc, &pwr, rtc::ClockSource::LSE, None).unwrap();
        rtc.enable_interrupts(rtc::Interrupts {
            wakeup_timer: true,
            ..rtc::Interrupts::default()
        });

        // Listen for RTC wakeup timer interrupt requests
        exti.listen_configurable(exti::ConfigurableLine::RtcWakeup, exti::TriggerEdge::Rising);

        // Start wakeup timer to update watch face every second
        rtc.wakeup_timer().start(1_u32);

        (
            Shared {
                pwr,
                rcc,
                scb: cp.SCB,
                buzzer_enabled: true,
                temperature: Temperature::default(),
                voltage: Voltage::default(),
            },
            Local { rtc, adc, buzzer },
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

    #[task(binds = RTC, local = [rtc], shared = [buzzer_enabled])]
    fn wakeup(cx: wakeup::Context) {
        defmt::info!("rtc wakeup");

        let rtc = cx.local.rtc;

        // Clear interrupt
        rtc.wakeup_timer().wait().unwrap();

        let time = rtc.now();

        // Sound the buzzer on the top of the hour if enabled
        use stm32l0xx_hal::rtc::Timelike as _;
        if *cx.shared.buzzer_enabled && time.minute() == 0 && time.second() == 0 {
            if let Err(()) = beep::spawn() {
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

    #[task(priority = 2, local = [adc], shared = [temperature, voltage])]
    async fn measure(cx: measure::Context) {
        defmt::info!("adc measure");
        let adc = cx.local.adc;

        let temperature = cx.shared.temperature;
        let voltage = cx.shared.voltage;

        let mut vtemp = VTemp::new();
        let mut vref = VRef::new();

        vtemp.enable(adc);
        vref.enable(adc);

        adc.calibrate().unwrap();

        (temperature, voltage).lock(|temp, volt| {
            *temp = adc.read(&mut vtemp).unwrap();
            *volt = adc.read(&mut vref).unwrap();

            defmt::info!("Temperature {}°C, Battery voltage {}V", *temp, *volt);
        });
    }
}
