use arrayvec::{ArrayString, CapacityError};
use itoa;

pub struct State {
    set_voltage: f32,
    output_switch_state: bool,
    pub output_enabled: bool
}

impl State {
    pub fn new(output_switch_state: bool) -> Self {
        Self {
            set_voltage: 0.,
            output_switch_state,
            output_enabled: !output_switch_state
        }
    }


    pub fn output_voltage(&self) -> f32 {
        if self.output_enabled {
            self.set_voltage
        }
        else {
            0.
        }
    }

    pub fn set_output_switch_state(&mut self, new: bool) {
        self.output_switch_state = new;
        if new == false {
            self.output_enabled = true;
        }
    }

    pub fn set_voltage(&mut self, voltage: f32) {
        self.set_voltage = voltage;
    }

    pub fn get_display(&self) -> Result<ArrayString<[u8; 32]>, CapacityError<&str>> {
        let mut result = ArrayString::new();
        let mut buffer = itoa::Buffer::new();
        result.push_str(buffer.format((self.set_voltage * 1000.) as u16 ));
        result.push_str(" mV ");

        if self.output_enabled {
            if self.output_switch_state {
                result.push_str("On");
            }
            else {
                result.push_str("Off");
            }
        }
        else {
            result.push_str("Disabled");
        }

        Ok(result)
    }
}
