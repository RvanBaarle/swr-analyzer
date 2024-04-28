#[repr(u16)]
#[derive(Copy, Clone)]
pub enum CommandOp {
    SweepEnable = 1,
    SweepDisable = 2,
    SweepOneshot = 3,
    /// Send SetRFGen with 0 for disable, 1 for enable
    SetRFGen = 10,
    GenFrequency = 11,
    /// Negative values possible, encoded as :13-12345678\r (only 8 digits allowed)
    StepFrequency = 13,
    MaxStepFrequency = 14,
    StepTime = 15,
    
    NoiseFilter = 32,

    /// Blinked using 942, 945
    LedOff = 945,
    LedBlink = 942,
    /// Unknown, sent on exit after :11010000000\r
    Exit = 96,
}