use crate::system::System;
use stm32l0::stm32l0x3::{GPIOA, GPIOB, LCD, SYSCFG};

/// Liquid crystal display
pub struct Lcd(LCD);

impl Lcd {
    pub fn configure(
        lcd: LCD,
        sys: &mut System,
        syscfg: &mut SYSCFG,
        gpioa: &mut GPIOA,
        gpiob: &mut GPIOB,
    ) -> Self {
        sys.enable_lcd_clk();

        // Configure comm pins
        gpioa
            .afrh
            .modify(|_, w| w.afsel8().af1().afsel9().af1().afsel10().af1());

        // Configure segment pins
        gpioa
            .afrl
            .modify(|_, w| w.afsel3().af1().afsel6().af1().afsel7().af1());

        gpioa.afrh.modify(|_, w| w.afsel15().af1());

        gpiob.afrl.modify(|_, w| {
            w.afsel0()
                .af1()
                .afsel1()
                .af1()
                .afsel3()
                .af1()
                .afsel4()
                .af1()
                .afsel5()
                .af1()
        });

        gpiob.afrh.modify(|_, w| {
            w.afsel8()
                .af1()
                .afsel10()
                .af1()
                .afsel11()
                .af1()
                .afsel12()
                .af1()
                .afsel13()
                .af1()
                .afsel14()
                .af1()
                .afsel15()
                .af1()
        });

        // Enable VLCD2 decouple capacitor on PB2
        syscfg
            .cfgr2
            .modify(|r, w| unsafe { w.bits((r.bits() & !(0x1F << 1)) | (1 << 1)) });

        // Configure the LCD frame control register
        //
        // * Set the frame rate to 31.03 Hz
        // * Set the LCD voltage to 3.12v
        // * Set pulse duration to 1/clk_pos FIXME: probably needs changing
        //
        // TODO: Figure out the best VLCD voltage for contrast
        lcd.fcr
            .write(|w| unsafe { w.ps().bits(4).div().bits(6).cc().bits(4).pon().bits(1) });

        // Configure the LCD control register
        //
        // * Set bias to 1/2
        // * Set duty to 1/3
        // * Use internal voltage source
        // * Enable LCD module
        lcd.cr.write(|w| unsafe {
            w.bias()
                .bits(0b001)
                .duty()
                .bits(0b010)
                .vsel()
                .clear_bit()
                .lcden()
                .set_bit()
        });

        Self(lcd)
    }

    /// Enable a specific segment
    pub unsafe fn enable_segment(&mut self, comm: usize, segment: usize) {
        // Get comm display memory register
        let reg = self.0.ram_com0.as_ptr().add(0x4 * comm);

        // Enable the segment
        reg.write(reg.read() | 1 << segment)
    }

    /// Trigger a display update
    pub fn update(&mut self) {
        self.0.sr.modify(|_, w| w.udr().set_bit());
    }
}
