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
mod state;

use rtfm::{Threshold, app};


use cortex_m::asm;
use stm32f103xx_hal::prelude::*;
use stm32f103xx_hal::gpio::gpioa::{PA8, PA9, self};
use stm32f103xx_hal::gpio::gpiob::{PBx, self};
use stm32f103xx_hal::gpio::{Output, PushPull, Floating, Input, PullDown, PullUp};
use stm32f103xx_hal::timer::{Timer};
use stm32f103xx_hal::pwm;
use stm32f103xx_hal::time::Hertz;
use stm32f103xx::{TIM2, TIM3};
use stm32f103xx::{EXTI, NVIC};
use rt::ExceptionFrame;
use rtfm::Resource;

use state::State;


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
        static PWM: pwm::Pwm<stm32f103xx::TIM2, pwm::C1>;
        static LCD: Lcd;
        static KEYPAD: Keypad;
        static KEY_DELAY_TIMER: Timer<TIM3>;
        static OUTPUT_SENSOR: PA8<Input<PullUp>>;
        static STATE: State;
        static INTERRUPT_CONTROLLER: NVIC;
        static EXTI_CONTROLLER: EXTI;
    },

    idle: {
        resources: [KEYPAD, KEY_DELAY_TIMER, STATE, LCD, INTERRUPT_CONTROLLER]
    },

    tasks: {
        EXTI1: {
            path: state_changed,
            resources: [PWM, LCD, STATE]
        },

        EXTI9_5: {
            path: output_switch_changed,
            resources: [OUTPUT_SENSOR, INTERRUPT_CONTROLLER, STATE, EXTI_CONTROLLER]
        }
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

    // Disable the JTAG hardware to free up PB3 and 4
    afio.mapr.disable_jtag();

    // Timer used for preventing bouncy buttons
    let timer = Timer::tim3(p.device.TIM3, Hertz(100), clocks, &mut rcc.apb1);

    ////////////////////////////////////////////////////////////////////////////////
    //                              PWM
    ////////////////////////////////////////////////////////////////////////////////
    let pwm_pin = gpioa.pa0.into_alternate_push_pull(&mut gpioa.crl);
    let mut pwm = p.device.TIM2.pwm(pwm_pin, &mut afio.mapr, Hertz(10_000), clocks, &mut rcc.apb1);
    pwm.set_duty(0);
    pwm.enable();

    ////////////////////////////////////////////////////////////////////////////////
    //                              LCD
    ////////////////////////////////////////////////////////////////////////////////
    let delay = stm32f103xx_hal::delay::Delay::new(syst, clocks);

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
    //                              Keypad
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

    ////////////////////////////////////////////////////////////////////////////////
    //                          Output switch
    ////////////////////////////////////////////////////////////////////////////////
    let output_sensor = gpioa.pa8.into_pull_up_input(&mut gpioa.crh);

    // Mask exti8
    p.device.EXTI.imr.modify(|_r, w| w.mr8().set_bit());
    // Trigger on both falling and rising edge
    p.device.EXTI.rtsr.modify(|_r, w| w.tr8().set_bit());
    p.device.EXTI.ftsr.modify(|_r, w| w.tr8().set_bit());

    ////////////////////////////////////////////////////////////////////////////////
    //                          Other
    ////////////////////////////////////////////////////////////////////////////////
    let state = State::new(output_sensor.is_low());


    // Write the initial state to the LCD
    write_line(1, &mut lcd, &state.get_display().unwrap());

    init::LateResources {
        PWM: pwm,
        LCD: lcd,
        KEYPAD: keypad,
        KEY_DELAY_TIMER: timer,
        OUTPUT_SENSOR: output_sensor,
        STATE: state,
        INTERRUPT_CONTROLLER: p.core.NVIC,
        EXTI_CONTROLLER: p.device.EXTI,
    }
}

fn idle(t: &mut Threshold, mut r: idle::Resources) -> ! {
    let mut last_key = None;

    let mut interface_state = interface::State::Start;
    let message = interface_state.get_display().unwrap();

    r.LCD.claim_mut(t, |lcd, _t| {
        write_line(0, lcd, &message);
    });

    loop {
        let key = r.KEYPAD.read_first_key();

        match key {
            Some(coords) => {
                let key_char = keypad::translate_coordinate(coords, &keymap::KEYMAP);

                if Some(key_char) != last_key {
                    // Process the key
                    let (new_state, command) = interface_state.update(key_char);
                    interface_state = new_state;

                    if let Some(interface::Command::Voltage(val)) = command {
                        r.STATE.claim_mut(t, |state, _t| {
                            state.set_voltage(val);
                        });
                        r.INTERRUPT_CONTROLLER.claim_mut(t, |nvic, _t| {
                            nvic.set_pending(stm32f103xx::Interrupt::EXTI1);
                        });
                    }

                    let message = interface_state.get_display().unwrap();

                    r.LCD.claim_mut(t, |lcd, _t| {
                        write_line(0, lcd, &message);
                    });

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

fn state_changed(t: &mut Threshold, mut r: EXTI1::Resources) {
    let min_voltage: f32 = 1.291;
    let max_voltage: f32 = 18.95 + min_voltage;
    let voltage_multiplyer = 1.046;

    let duty_percentage = voltage::pwm_percentage_for_voltage(
        r.STATE.output_voltage(),
        min_voltage,
        max_voltage
    );

    // Write the current status
    write_line(1, &mut r.LCD, &r.STATE.get_display().unwrap());

    let duty = (r.PWM.get_max_duty() as f32) * duty_percentage * voltage_multiplyer;
    r.PWM.set_duty(duty as u16);
}


fn output_switch_changed(t: &mut Threshold, mut r: EXTI9_5::Resources) {
    r.STATE.set_output_switch_state(r.OUTPUT_SENSOR.is_low());

    // Clear this interrupt and raise the state change interrupt
    r.INTERRUPT_CONTROLLER.set_pending(stm32f103xx::Interrupt::EXTI1);
    r.EXTI_CONTROLLER.pr.modify(|_, w| w.pr8().set_bit());
}


/**
  Writes a single line to the LCD with padding to clear the previous content
*/
fn write_line(line: u8, lcd: &mut Lcd, message: &str) {
    if line == 0 {
        lcd.set_cursor_pos(0);
    }
    else {
        lcd.set_cursor_pos(40);
    };
    const LINE_LENGTH: usize = 16;
    let amount_of_padding = LINE_LENGTH - message.len();

    lcd.write_str(message);
    for _ in 0..amount_of_padding {
        lcd.write_char(' ');
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
