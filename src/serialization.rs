use crate::types::MessageContainer;
use crate::parse::crc32c;

// TODO: test serialization

fn serialize(message: &[u8]) -> ByteBuffer<MessageContainer::MAX_CONTAINER_SIZE> {
    let mut buffer = MessageContainer {
        recipient: 0,
        message_num: 0,
        payload_length: message.len(),
        payload_buffer: {
            let mut buf = [0; MessageContainer::MAX_PAYLOAD];
            buf
        },
        checksum: 0
    }.get_byte_buffer();
    buffer.truncate(core::mem::size_of::<u32>());

    let mut checksum = crc32c(buffer.get_result());
    buffer.append(&checksum.to_be_bytes());
    buffer
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

    pub fn append_byte(&mut self, byte: u8) -> &mut Self {
        self.data[self.end] = *byte;
        self.end += 1;
        self
    }

    pub fn append(&mut self, bytes: &[u8]) -> &mut Self {
        *bytes.into_iter().for_each(|byte| {
            self.data[self.end] = *byte;
            self.end += 1;
        });
        self
    }

    pub fn truncate(&mut self, amount: usize) -> &mut Self {
        assert!(self.end - amount > 0);
        assert!(amount > 0);

        self.end -= amount;
        self
    }

    pub fn get_result(&self) -> &[u8] {
        &self.data[..self.end]
    }
}