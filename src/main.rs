#![no_std]
#![no_main]

#[macro_use]
extern crate cortex_m_rt as rt;
extern crate cortex_m;
extern crate cortex_m_rtfm as rtfm;
extern crate cortex_m_semihosting;
extern crate embedded_hal as hal;
extern crate panic_semihosting;
extern crate stm32f103xx_hal;
extern crate stm32f103xx;
extern crate hd44780_driver;

use rtfm::{Threshold, app};


use cortex_m::asm;
use stm32f103xx_hal::prelude::*;
use stm32f103xx_hal::gpio::gpioa::{self, PA8};
use stm32f103xx_hal::gpio::gpiob;
use stm32f103xx_hal::gpio::{Alternate, Output, PushPull, Floating, Input};
use stm32f103xx_hal::pwm;
use stm32f103xx_hal::time::Hertz;
use stm32f103xx_hal::delay::Delay;
use rt::ExceptionFrame;
use rtfm::Resource;
use hd44780_driver::{FourBitBus, HD44780};

type LcdType = HD44780<Delay, FourBitBus<
            gpiob::PB4<Output<PushPull>>,
            gpiob::PB3<Output<PushPull>>,
            gpioa::PA15<Output<PushPull>>,
            gpioa::PA12<Output<PushPull>>,
            gpioa::PA11<Output<PushPull>>,
            gpioa::PA10<Output<PushPull>>
        >>;

app! {
    device: stm32f103xx,

    resources: {
        static LED: PA8<Output<PushPull>>;
        static PWM: pwm::Pwm<stm32f103xx::TIM2, pwm::C1>;
        static LCD: LcdType;
    },

    idle: {
        resources: [LED, PWM, LCD]
    }
}


fn init(p: init::Peripherals) -> init::LateResources {
    let mut flash = p.device.FLASH.constrain();
    let mut rcc = p.device.RCC.constrain();
    let clocks = rcc.cfgr.freeze(&mut flash.acr);
    let mut gpioa = p.device.GPIOA.split(&mut rcc.apb2);
    let mut gpiob = p.device.GPIOB.split(&mut rcc.apb2);
    let mut afio = p.device.AFIO.constrain(&mut rcc.apb2);
    let syst = p.core.SYST;

    let led = gpioa.pa8.into_push_pull_output(&mut gpioa.crh);


    let pwm_pin = gpioa.pa0.into_alternate_push_pull(&mut gpioa.crl);
    let mut pwm = p.device.TIM2.pwm(pwm_pin, &mut afio.mapr, Hertz(20_000), clocks, &mut rcc.apb1);
    pwm.set_duty(512);
    pwm.enable();

    let mut lcd = HD44780::new_4bit(
            gpiob.pb4.into_push_pull_output(&mut gpiob.crl), // Reset pin
            gpiob.pb3.into_push_pull_output(&mut gpiob.crl), // Enable pin

            gpioa.pa15.into_push_pull_output(&mut gpioa.crh), // D4
            gpioa.pa12.into_push_pull_output(&mut gpioa.crh), // D5
            gpioa.pa11.into_push_pull_output(&mut gpioa.crh), // D6
            gpioa.pa10.into_push_pull_output(&mut gpioa.crh), // D7

            stm32f103xx_hal::delay::Delay::new(syst, clocks)
        );


    init::LateResources {
        LED: led,
        PWM: pwm,
        LCD: lcd
    }
}

fn idle(t: &mut Threshold, r: idle::Resources) -> ! {
    r.LCD.reset();
    r.LCD.clear();
    r.LCD.set_display_mode(true, true, true);
    r.LCD.write_str("Hello, world");
    r.LCD.set_cursor_pos(40);
    r.LCD.write_str("Next line");

    r.LED.set_high();

    loop {
        asm::wfi()
    }
}


exception!(HardFault, hard_fault);

fn hard_fault(ef: &ExceptionFrame) -> ! {
    panic!("{:#?}", ef);
}

exception!(*, default_handler);

fn default_handler(irqn: i16) {
    panic!("Unhandled exception (IRQn = {})", irqn);
}
