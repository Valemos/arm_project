//! Sensor message types

use crate::serialization::ByteBuffer;

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


pub struct MessageContainer {
    pub recipient: u8,
    pub message_num: u8,
    pub payload_length: usize,
    pub payload_buffer: [u8; Self::MAX_PAYLOAD],
    pub checksum: u32,
}

impl MessageContainer {
    pub const PAYLOAD_LENGTH_BYTES: usize = 2;
    pub const MAX_PAYLOAD: usize = 200;
    pub const MAX_CONTAINER_SIZE: usize = core::mem::size_of::<MessageContainer>();

    pub fn get_byte_buffer(&self) -> ByteBuffer<{ MessageContainer::MAX_CONTAINER_SIZE }> {
        let mut buffer = ByteBuffer::<MessageContainer::MAX_CONTAINER_SIZE>::new();

        buffer.append_byte(self.recipient)
            .append_byte(self.message_num)
            .append(&self.payload_length.to_be_bytes())
            .append(&self.payload_buffer[..self.payload_length])
            .append(&self.checksum.to_be_bytes());

        buffer
    }

    pub fn get_payload<'a>(&self) -> &'a[u8] {
        &self.payload_buffer[0..self.payload_length]
    }
}

impl Default for MessageContainer {
    fn default() -> Self {
        MessageContainer {
            recipient: 0,
            message_num: 0,
            payload_length: 0,
            payload_buffer: [0; Self::MAX_PAYLOAD],
            checksum: 0,
        }
    }
}
