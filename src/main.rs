#![no_main]
#![no_std]
#![feature(type_alias_impl_trait)]

use defmt_rtt as _; // global logger
use panic_probe as _; // panic handler

#[rtic::app(
    device = stm32l0xx_hal::pac,
    dispatchers = []
)]
mod app {
    use stm32l0xx_hal::prelude::*;
    use stm32l0xx_hal::{pwr, rcc, rtc, exti};
    use cortex_m::peripheral::SCB;

    #[shared]
    struct Shared {
        pwr: pwr::PWR,
        scb: SCB,
        rcc: rcc::Rcc,
    }

    #[local]
    struct Local {
    }

    #[init]
    fn init(cx: init::Context) -> (Shared, Local) {
        defmt::info!("init");

        let dp = cx.device;
        let cp = cx.core;

        // Set clock to MSI range 0 (low power consumption)
        let mut rcc = dp.RCC.freeze(rcc::Config::msi(rcc::MSIRange::Range0));

        let pwr = pwr::PWR::new(dp.PWR, &mut rcc);
        let mut exti = exti::Exti::new(dp.EXTI);

        let mut rtc = rtc::Rtc::new(dp.RTC, &mut rcc, &pwr, None).unwrap();

        rtc.enable_interrupts(rtc::Interrupts {
            wakeup_timer: true,
            ..rtc::Interrupts::default()
        });

        exti.listen_configurable(exti::ConfigurableLine::RtcWakeup, exti::TriggerEdge::Rising);

        // Start wakeup timer to update watch face every second
        rtc.wakeup_timer().start(1_u32);

        (
            Shared {
                pwr,
                rcc,
                scb: cp.SCB,
            },
            Local {
            },
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
            pwr.stop_mode(&mut scb, &mut rcc, pwr::StopModeConfig { ultra_low_power: true }).enter();
        });

        // Should never be reached
        loop {}
    }
}
