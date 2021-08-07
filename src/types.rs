//! Sensor message types

/// Messages sent by the sensor
#[derive(Copy, Clone, Debug)]
pub enum SensorMessage {
    SensorHealth(SensorHealthMessage),
    PrimaryOpticalMessage(PrimaryOpticalMessage),
    EnvironmentalMessage(EnvironmentalMessage),
}

/// Sensor version and any alertable conditions, such as over-temperature.
///
/// Warning: The data specified here is just a sketch and may differ significantly from production code.
#[derive(Copy, Clone, Debug)]
pub struct SensorHealthMessage {
    software_commit: [u8; 20],
    hardware_version: (u8, u8, u8),
    temperature_ok: bool,
} // TODO: Populate

/// LED measurements
#[derive(Copy, Clone, Debug)]
pub struct PrimaryOpticalMessage {
    passive: [u16; Self::LENGTH],
    active: [u16; Self::LENGTH],
}
impl PrimaryOpticalMessage {
    pub const LENGTH: usize = 10;
}

/// Environmental readings from the thermometer, accelerometer etc.
#[derive(Copy, Clone, Debug)]
pub struct EnvironmentalMessage {
    acc_x: f32,
    acc_y: f32,
    acc_z: f32,
    temp: f32,
}
