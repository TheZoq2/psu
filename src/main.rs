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
extern crate itoa;

mod voltage;
mod keypad;
mod keymap;

use rtfm::{Threshold, app};


use stm32f103xx_hal::prelude::*;
use stm32f103xx_hal::gpio::gpioa::{PA8, self, PAx};
use stm32f103xx_hal::gpio::{gpiob};
use stm32f103xx_hal::gpio::{Output, PushPull, Floating, Input, PullDown};
use stm32f103xx_hal::pwm;
use stm32f103xx_hal::time::Hertz;
use rt::ExceptionFrame;

type Lcd = hd44780_driver::HD44780<
    // Delay
    stm32f103xx_hal::delay::Delay,
    hd44780_driver::FourBitBus<
        // Reset pin
        gpioa::PA10<Output<PushPull>>,
        // Enable pin
        gpioa::PA9<Output<PushPull>>,
        // D4
        gpiob::PB15<Output<PushPull>>,
        // D5
        gpiob::PB14<Output<PushPull>>,
        // D6
        gpiob::PB13<Output<PushPull>>,
        // D7
        gpiob::PB12<Output<PushPull>>,
    >,
>;

type KeypadInput = PAx<Input<PullDown>>;
type KeypadOutput = PAx<Output<PushPull>>;
type Keypad = keypad::Keypad<[KeypadInput; 3], [KeypadOutput; 4], KeypadInput, KeypadOutput>;


app! {
    device: stm32f103xx,

    resources: {
        static LED: PA8<Output<PushPull>>;
        static PWM: pwm::Pwm<stm32f103xx::TIM2, pwm::C1>;
        static LEFT_PIN: gpiob::PB10<Input<Floating>>;
        static RIGHT_PIN: gpiob::PB11<Input<Floating>>;
        static LCD: Lcd;
        static KEYPAD: Keypad;
    },

    idle: {
        resources: [LED, PWM, LEFT_PIN, RIGHT_PIN, LCD, KEYPAD]
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


    let delay = stm32f103xx_hal::delay::Delay::new(syst, clocks);


    let pwm_pin = gpioa.pa0.into_alternate_push_pull(&mut gpioa.crl);
    let mut pwm = p.device.TIM2.pwm(pwm_pin, &mut afio.mapr, Hertz(20_000), clocks, &mut rcc.apb1);
    pwm.set_duty(128);
    pwm.enable();

    ////////////////////////////////////////////////////////////////////////////////
    // LCD
    ////////////////////////////////////////////////////////////////////////////////
    let mut lcd = hd44780_driver::HD44780::new_4bit(
            // rs
            gpioa.pa10.into_push_pull_output(&mut gpioa.crh),
            // en
            gpioa.pa9.into_push_pull_output(&mut gpioa.crh),
            // d4
            gpiob.pb15.into_push_pull_output(&mut gpiob.crh),
            // d5
            gpiob.pb14.into_push_pull_output(&mut gpiob.crh),
            // d6
            gpiob.pb13.into_push_pull_output(&mut gpiob.crh),
            // d7
            gpiob.pb12.into_push_pull_output(&mut gpiob.crh),
            // Delay
            delay
        );
    lcd.clear();
    lcd.set_display_mode(true, false, false);
    lcd.write_str("Hello, world!");

    ////////////////////////////////////////////////////////////////////////////////
    // Keypad
    ////////////////////////////////////////////////////////////////////////////////

    let keypad = Keypad::new(
            [
                gpioa.pa5.into_pull_down_input(&mut gpioa.crl).downgrade(),
                gpioa.pa6.into_pull_down_input(&mut gpioa.crl).downgrade(),
                gpioa.pa7.into_pull_down_input(&mut gpioa.crl).downgrade(),
            ],
            [
                gpioa.pa4.into_push_pull_output(&mut gpioa.crl).downgrade(),
                gpioa.pa3.into_push_pull_output(&mut gpioa.crl).downgrade(),
                gpioa.pa2.into_push_pull_output(&mut gpioa.crl).downgrade(),
                gpioa.pa1.into_push_pull_output(&mut gpioa.crl).downgrade(),
            ]
        );


    init::LateResources {
        LED: led,
        PWM: pwm,
        LEFT_PIN: gpiob.pb10.into_floating_input(&mut gpiob.crh),
        RIGHT_PIN: gpiob.pb11.into_floating_input(&mut gpiob.crh),
        LCD: lcd,
        KEYPAD: keypad
    }
}

fn idle(_t: &mut Threshold, r: idle::Resources) -> ! {
    r.LED.set_high();

    let min_voltage: f32 = 1.25;
    let max_voltage: f32 = 20.25;
    let mut voltage: f32 = 5.;

    loop {
        let old_voltage = voltage;
        if r.LEFT_PIN.is_low() {
            voltage += 0.1;
            while r.LEFT_PIN.is_low() {}
        }
        if r.RIGHT_PIN.is_low() {
            voltage -= 0.1;
            while r.RIGHT_PIN.is_low() {}
        }

        if voltage < min_voltage {
            voltage = min_voltage
        }
        if voltage > max_voltage {
            voltage = max_voltage;
        }

        if old_voltage != voltage {
            r.LCD.clear();
            let mut buffer = itoa::Buffer::new();
            let voltage_string = buffer.format((voltage * 1000.) as i32);
            r.LCD.write_str(voltage_string);
            r.LCD.write_str(" mV");

            let duty = (r.PWM.get_max_duty() as f32) * voltage::pwm_percentage_for_voltage(
                voltage,
                min_voltage,
                max_voltage
            );
            let mut buffer = itoa::Buffer::new();
            let pwm_string = buffer.format(duty as u16);
            r.LCD.set_cursor_pos(40);
            r.LCD.write_str("PWM: ");
            r.LCD.write_str(pwm_string);
            r.PWM.set_duty(duty as u16);
        }


        {
            let key = r.KEYPAD.read_first_key();

            key.map(|k| {
                let translated = keypad::translate_coordinate(
                    k,
                    &keymap::KEYMAP
                );

                r.LCD.set_cursor_pos(10);
                r.LCD.write_char(translated);
            });

        }
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
