//! Code for parsing sensor messages from a stream of bytes.
//!
//! TODO: Doctest demonstrating how to serialise and parse messages.


use crate::parse::ParserNeeds::{Finished, Prefix};
use crate::serialization::ByteBuffer;
use crate::types::{MessageContainer, PayloadType};
use std::io::Read;

#[cfg(test)]
pub mod test;


/// Obscures the encoding.
///
/// Note: Frobnication is not encryption; the goal is not to make it impossible for an attacker to
/// read the data.  The goal is to make it hard to accidentally interpret the data as something
/// else.  For example, if a field value consistently matches the start sequence, this could cause
/// synchronisation to fail.  Frobnication protects against this.
pub fn frobnicate(data: &mut [u8], mut seed: u8) {
    fn step(seed: u8) -> u8 {
        (seed << 1) ^ (if (seed >> 7) != 0 { 0x69 } else { 0 })
    }
    seed ^= 0x42;
    for _ in 0..8 {
        seed = step(seed);
    }
    for byte in data {
        seed = step(seed);
        *byte ^= seed;
    }
}

/// Result of shifting out one byte on a CRC32C.
fn crc32c_byte(byte: u8) -> u32 {
    (0..8).fold(byte as u32, |acc, _| match acc & 1 {
        1 => 0x82F63B78 ^ (acc >> 1),
        _ => acc >> 1,
    })
}

/// Checksum of bytes, using CRC32C.
pub fn crc32c(bytes: &[u8]) -> u32 {
    crc32c_update(0, bytes)
}

pub fn crc32c_update(seed: u32, bytes: &[u8]) -> u32 {
    !bytes.iter().fold(!seed, |acc, octet| {
        (acc >> 8) ^ crc32c_byte((acc as u8) ^ *octet)
    })
}

const PREFIX: [u8; 4] = [0xaa, 0xaa, 0x55, 0x55];

#[derive(Debug, PartialEq)]
enum ParserNeeds {
    Prefix(usize),
    Recipient,
    Counter,
    Length(usize),
    Payload(usize),
    Checksum(usize),
    Finished,
}
impl Default for ParserNeeds {
    fn default() -> Self {
        ParserNeeds::Prefix(0)
    }
}

/// Reads a stream of bytes and emits a stream of messages.
///
/// This is implemented as a state machine, where the state corresponds to the next byte expected from the input.  If the next byte is incompatible with the message format, the bytes read so far are discarded and the parser seraches for the start of the next message.
///
pub struct Parser<T: Read> {
    byte_stream: T,
    parsed: MessageContainer,
    state: ParserNeeds,
}

impl<T: Read> Parser<T> {

    pub fn from_stream(byte_stream: T) -> Parser<T> {
        Parser {
            byte_stream,
            parsed: MessageContainer::default(),
            state: Prefix(0)
        }
    }

    pub fn try_read_next(&mut self) -> Result<PayloadType, ()> {

        self.state = self.reset();
        for byte in self.byte_stream.by_ref().bytes() {
            match byte {
                Ok(byte) => {
                    self.step(byte);
                    if self.state == Finished {
                        return Ok(self.parsed.payload_buffer.clone());
                    }
                }
                Err(_) => return Err(()),
            }
        }

        Err(())
    }

    pub fn step(&mut self, byte: u8) {
        use ParserNeeds::*;

        self.parsed.checksum = (self.parsed.checksum >> 8)
                                ^ crc32c_byte((self.parsed.checksum as u8) ^ byte);

        self.state = match self.state {
            Prefix(index) => {
                if byte == PREFIX[index] {
                    if index + 1 == PREFIX.len() {
                        Ok(Recipient)
                    } else {
                        Ok(Prefix(index + 1))
                    }
                } else {
                    Err(())
                }
            }
            Recipient => {
                self.parsed.recipient = byte;
                Ok(Counter)
            }
            Counter => {
                self.parsed.message_num = byte;
                Ok(Length(0))
            }
            Length(index) => {
                self.parsed.payload_length = if index == 0 {
                    byte as usize
                } else {
                    self.parsed.payload_length + ((byte as usize) << index)
                };
                if self.parsed.payload_length > MessageContainer::MAX_PAYLOAD {
                    Err(())
                } else if index + 1 < MessageContainer::PAYLOAD_LENGTH_BYTES {
                    Ok(Length(index + 1))
                } else {
                    Ok(Payload(0))
                }
            }
            Payload(index) => {
                self.parsed.payload_buffer[index] = byte;
                if index + 1 < self.parsed.payload_length {
                    Ok(Payload(index + 1))
                } else {
                    Ok(Checksum(0))
                }
            }
            Checksum(index) => {
                if byte != take_byte_u32(self.parsed.checksum, index) {
                    Err(())
                } else if index < 3 {
                    Ok(Checksum(index + 1))
                } else {
                    Ok(Finished)
                }
            }
            Finished => {
                Ok(self.reset())
            }
        }
        .unwrap_or_else(|_| self.reset());
    }

    fn reset(&mut self) -> ParserNeeds {
        self.parsed.checksum = 0;
        self.parsed.payload_buffer = [0; MessageContainer::MAX_PAYLOAD];
        ParserNeeds::Prefix(0)
    }
}
impl<T: Read> IntoIterator for Parser<T> {
    type Item = PayloadType;
    type IntoIter = ParserIterator<T>;

    fn into_iter(mut self) -> Self::IntoIter {
        ParserIterator { parser: self}
    }
}

struct ParserIterator<T: Read> {
    parser: Parser<T>,
}
impl<T: Read> Iterator for ParserIterator<T> {
    type Item = PayloadType;

    fn next(&mut self) -> Option<Self::Item> {
        match self.parser.try_read_next() {
            Ok(message) => { Some(message) }
            Err(_) => { None }
        }
    }
}

#[inline(always)]
fn take_byte_u32(number: u32, index: usize) -> u8 {
    (number >> (8 * (3 - index))) as u8
}
