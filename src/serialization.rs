use crate::types::MessageContainer;
use crate::parse::crc32c;


pub fn serialize(message: &[u8]) -> ByteBuffer<{ MessageContainer::MAX_CONTAINER_SIZE }> {
    let mut buffer = MessageContainer {
        recipient: 0,
        message_num: 0,
        payload_length: message.len(),
        payload_buffer: {
            let mut buf = [0; MessageContainer::MAX_PAYLOAD];
            message.iter().enumerate().for_each(|(i, byte)| buf[i] = *byte);
            buf
        },
        checksum: 0
    }.get_byte_buffer();
    buffer = buffer.truncate(core::mem::size_of::<u32>());

    let checksum = crc32c(buffer.get_result());
    buffer.append(&checksum.to_be_bytes())
}

pub struct ByteBuffer<const SIZE: usize>{
    data: [u8; SIZE],
    end: usize,
}
impl<const SIZE: usize> ByteBuffer<SIZE> {
    pub fn new() -> ByteBuffer<SIZE> {
        ByteBuffer {
            data: [0; SIZE],
            end: 0,
        }
    }

    pub fn append_byte(mut self, byte: u8) -> Self {
        self.data[self.end] = byte;
        self.end += 1;
        self
    }

    pub fn append(mut self, bytes: &[u8]) -> Self {
        bytes.iter().for_each(|byte| {
            self.data[self.end] = *byte;
            self.end += 1;
        });
        self
    }

    pub fn truncate(mut self, amount: usize) -> Self {
        assert!(self.end - amount > 0);
        assert!(amount > 0);

        self.end -= amount;
        self
    }

    pub fn get_result(&self) -> &[u8] {
        &self.data[..self.end]
    }
}