pub fn pwm_percentage_for_voltage(target: f32, min_voltage: f32, max_voltage: f32) -> f32 {
    (target - min_voltage) / (max_voltage - min_voltage)
}
