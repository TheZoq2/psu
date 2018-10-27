use itoa;
use arrayvec::{CapacityError, ArrayString};


#[derive(Clone, Debug, PartialEq)]
pub enum Command {
    Voltage(f32),
    Current(f32),
    OutputOn,
    OutputOff
}

#[derive(Clone, Debug, PartialEq)]
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
            (State::Confirm(_), '2') => (State::Start, None),


            // Toggle output
            (State::ToggleOutput, '1') => (State::Start, Some(Command::OutputOn)),
            (State::ToggleOutput, '2') => (State::Start, Some(Command::OutputOff)),


            (state, _) => (state, None),
        }
    }

    pub fn get_display(&self) -> Result<ArrayString<[u8; 32]>, CapacityError<&str>> {
        match *self {
            State::Start => {
                ArrayString::from("1:V 2:A 3:I/o")
            }
            State::InputCurrent(val) => {
                let mut result = ArrayString::new();
                let mut buffer = itoa::Buffer::new();
                result.push_str(buffer.format(val));
                result.push_str(" mA");
                Ok(result)
            }
            State::InputVoltage(val) => {
                let mut result = ArrayString::new();
                let mut buffer = itoa::Buffer::new();
                result.push_str(buffer.format(val));
                result.push_str(" mV");
                Ok(result)
            }
            State::Confirm(_) => {
                ArrayString::from("Confirm 1:y 2:n")
            }
            State::ToggleOutput => {
                ArrayString::from("1:On 2:Off")
            }
        }
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


#[cfg(test)]
mod tests {
    use super::*;

    fn run_input_sequence(seq: &str, initial_state: State) -> (State, Option<Command>)
    {
        let mut state = initial_state;
        let mut last_cmd = None;
        for input in seq.chars() {
            let (new_state, new_cmd) = state.update(input);
            state = new_state;
            last_cmd = new_cmd;
        }

        (state, last_cmd)
    }

    #[test]
    fn voltage_input() {
        assert_eq!(
            run_input_sequence("11234a1", State::Start),
            (State::Start, Some(Command::Voltage(1.234)))
        );
    }
    #[test]
    fn current_input() {
        assert_eq!(
            run_input_sequence("22", State::Start),
            (State::InputCurrent(2), None)
        );
        assert_eq!(
            run_input_sequence("22a", State::Start),
            (State::Confirm(Command::Current(0.002)), None)
        );
        assert_eq!(
            run_input_sequence("2234a1", State::Start),
            (State::Start, Some(Command::Current(0.234)))
        );
    }
    #[test]

    fn aborted_voltage() {
        assert_eq!(
            run_input_sequence("11234a2", State::Start),
            (State::Start, None)
        );
    }
    #[test]
    fn aborted_current() {
        assert_eq!(
            run_input_sequence("2234a2", State::Start),
            (State::Start, None)
        );
    }

    #[test]
    fn toggle_output() {
        assert_eq!(
            run_input_sequence("31", State::Start),
            (State::Start, Some(Command::OutputOn))
        );
        assert_eq!(
            run_input_sequence("32", State::Start),
            (State::Start, Some(Command::OutputOff))
        );
    }
}
