#[derive(Clone)]
pub enum Command {
    Voltage(f32),
    Current(f32),
    OutputOn,
    OutputOff
}

#[derive(Clone)]
pub enum State {
    Start,
    InputVoltage(u16),
    Confirm(Command),
    InputCurrent(u16),
    ToggleOutput
}

impl State {
    pub fn update(self, input: char) -> (Self, Option<Command>) {
        match (self, input) {
            // Start state
            (State::Start, '1') => (State::InputVoltage(0), None),
            (State::Start, '2') => (State::InputCurrent(0), None),
            (State::Start, '3') => (State::ToggleOutput, None),

            // Voltage input
            (State::InputVoltage(_), 'b') => (State::Start, None),
            (State::InputVoltage(val), 'a') => {
                let voltage = (val as f32) / 1000.;
                (State::Confirm(Command::Voltage(voltage)), None)
            }
            (State::InputVoltage(val), _) => {
                (State::InputVoltage(add_digit(val, input)), None)
            }

            // Current input
            (State::InputCurrent(_), 'b') => (State::Start, None),
            (State::InputCurrent(val), 'a') => {
                let current = (val as f32) / 1000.;
                (State::Confirm(Command::Current(current)), None)
            }
            (State::InputCurrent(val), _) => {
                (State::InputCurrent(add_digit(val, input)), None)
            }

            // Confirm
            (State::Confirm(cmd), '1') => (State::Start, Some(cmd)),
            (State::Confirm(cmd), '2') => (State::Start, None),


            // Toggle output
            (State::ToggleOutput, '1') => (State::Start, Some(Command::OutputOn)),
            (State::ToggleOutput, '2') => (State::Start, Some(Command::OutputOff)),


            (state, _) => (state, None),
        }
    }

    pub fn get_display(&self, buffer: &mut str) -> usize {
        unimplemented!()
    }
}


fn add_digit(val: u16, digit: char) -> u16 {
    match char_to_num(digit) {
        Some(digit) => val * 10 + (digit as u16),
        None => val
    }
}

// Might be better to use a lib for this
fn char_to_num(digit: char) -> Option<u8>{
    if digit.is_digit(10) {
        Some(digit as u8 - 48)
    }
    else {
        None
    }
}
