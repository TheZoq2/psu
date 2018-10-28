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
extern crate arrayvec;
#[macro_use]
extern crate nb;

mod voltage;
mod keypad;
mod keymap;
mod interface;

use rtfm::{Threshold, app};


use cortex_m::asm;
use stm32f103xx_hal::prelude::*;
use stm32f103xx_hal::gpio::gpioa::{PA8, self};
use stm32f103xx_hal::gpio::gpiob::{PBx, self};
use stm32f103xx_hal::gpio::{Output, PushPull, Floating, Input, PullDown};
use stm32f103xx_hal::timer::{Timer};
use stm32f103xx_hal::pwm;
use stm32f103xx_hal::time::Hertz;
use rt::ExceptionFrame;

use stm32f103xx::{TIM2, TIM3};

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

type KeypadInput = PBx<Input<PullDown>>;
type KeypadOutput = PBx<Output<PushPull>>;
type Keypad = keypad::Keypad<[KeypadInput; 3], [KeypadOutput; 4], KeypadInput, KeypadOutput>;


app! {
    device: stm32f103xx,

    resources: {
        static LED: PA8<Output<PushPull>>;
        static PWM: pwm::Pwm<stm32f103xx::TIM2, pwm::C1>;
        static LCD: Lcd;
        static KEYPAD: Keypad;
        static KEY_DELAY_TIMER: Timer<TIM3>;
    },

    idle: {
        resources: [LED, PWM, LCD, KEYPAD, KEY_DELAY_TIMER]
        // resources: [LED, LCD, KEYPAD, KEY_DELAY_TIMER]
    },
}


fn init(p: init::Peripherals) -> init::LateResources {
    let mut flash = p.device.FLASH.constrain();
    let mut rcc = p.device.RCC.constrain();
    let clocks = rcc.cfgr.freeze(&mut flash.acr);
    let mut gpioa = p.device.GPIOA.split(&mut rcc.apb2);
    let mut gpiob = p.device.GPIOB.split(&mut rcc.apb2);
    let mut afio = p.device.AFIO.constrain(&mut rcc.apb2);
    let syst = p.core.SYST;

    afio.mapr.disable_jtag();

    let led = gpioa.pa8.into_push_pull_output(&mut gpioa.crh);

    let timer = Timer::tim3(p.device.TIM3, Hertz(100), clocks, &mut rcc.apb1);

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
                gpiob.pb9.into_pull_down_input(&mut gpiob.crh).downgrade(),
                gpiob.pb8.into_pull_down_input(&mut gpiob.crh).downgrade(),
                gpiob.pb7.into_pull_down_input(&mut gpiob.crl).downgrade(),
            ],
            [
                gpiob.pb6.into_push_pull_output(&mut gpiob.crl).downgrade(),
                gpiob.pb5.into_push_pull_output(&mut gpiob.crl).downgrade(),
                gpiob.pb4.into_push_pull_output(&mut gpiob.crl).downgrade(),
                gpiob.pb3.into_push_pull_output(&mut gpiob.crl).downgrade(),
            ]
        );


    init::LateResources {
        LED: led,
        PWM: pwm,
        LCD: lcd,
        KEYPAD: keypad,
        KEY_DELAY_TIMER: timer,
    }
}

fn idle(_t: &mut Threshold, r: idle::Resources) -> ! {
    let min_voltage: f32 = 1.293;
    let max_voltage: f32 = 18.93 + min_voltage;
    let voltage_multiplyer = 1.04;
    
    let mut last_key = None;

    let mut interface_state = interface::State::Start;

    loop {
        {
            let key = r.KEYPAD.read_first_key();

            match key {
                Some(coords) => {
                    let key_char = keypad::translate_coordinate(coords, &keymap::KEYMAP);

                    if Some(key_char) != last_key {
                        // Process the key
                        let (new_state, command) = interface_state.update(key_char);
                        interface_state = new_state;

                        if let Some(interface::Command::Voltage(val)) = command {
                            let duty_percentage = voltage::pwm_percentage_for_voltage(
                                val,
                                min_voltage,
                                max_voltage
                            );

                            let duty = ((r.PWM.get_max_duty() as f32) * duty_percentage * voltage_multiplyer);
                            r.PWM.set_duty(duty as u16)
                        }

                        r.LCD.clear();
                        r.LCD.write_str(&interface_state.get_display().unwrap());

                        last_key = Some(key_char)
                    }

                    r.KEY_DELAY_TIMER.start(Hertz(100));
                    block!(r.KEY_DELAY_TIMER.wait());
                }
                None => {
                    last_key = None
                }
            }
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
