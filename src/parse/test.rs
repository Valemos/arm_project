use super::*;

#[test]
fn frobnicate_should_encode_correctly() {
    let test_vectors: Vec<(Vec<u8>, u8, Vec<u8>, String)> = vec![
        (vec![], 0, vec![], "Empty".to_string()),
        (vec![], 1, vec![], "Empty with various seeds".to_string()),
        (vec![], 0xff, vec![], "Empty with various seeds".to_string()),
        (vec![0], 0, vec![0x78], "Nowrap".to_string()),
        (vec![0], 1, vec![0xaa], "Wrap".to_string()),
        (
            vec![0, 0, 0, 0, 0, 0, 0, 0],
            0,
            vec![0x78, 0xf0, 0x89, 0x7b, 0xf6, 0x85, 0x63, 0xc6],
            "Blank2".to_string(),
        ),
        (
            vec![0, 0, 0, 0, 0, 0, 0, 0],
            1,
            vec![0xaa, 0x3d, 0x7a, 0xf4, 0x81, 0x6b, 0xd6, 0xc5],
            "Blank2Seed1".to_string(),
        ),
        (
            vec![1, 1, 1, 1, 1, 1, 1, 1],
            0,
            vec![0x79, 0xf1, 0x88, 0x7a, 0xf7, 0x84, 0x62, 0xc7],
            "Ones2".to_string(),
        ),
    ];
    for (inbytes, seed, expected, name) in test_vectors {
        let mut actual = inbytes.clone();
        frobnicate(&mut actual, seed);
        assert_eq!(
            &actual, &expected,
            "Frobnicated is not as expected for {}",
            name
        );
        frobnicate(&mut actual, seed);
        assert_eq!(
            &actual, &inbytes,
            "Frobnicating twice did not return the original data for {}",
            name
        );
    }
}

#[cfg(test)]
mod crc32c_tests {
    use super::crc32c;

    /// http://reveng.sourceforge.net/crc-catalogue/17plus.htm#crc.cat.crc-32c
    #[test]
    fn crc_catalog() {
        assert_eq!(0xe3069283, crc32c(b"123456789"))
    }

    /// IETF test vectors: https://datatracker.ietf.org/doc/html/rfc3720#appendix-B.4
    #[test]
    fn rfc3270_all_zeros() {
        assert_eq!(0x8a9136aa, crc32c(&vec![0; 32]))
    }

    #[test]
    fn rfc3270_all_ones() {
        assert_eq!(0x62a8ab43, crc32c(&vec![0xff; 32]))
    }

    #[test]
    fn rfc3270_increasing_values() {
        assert_eq!(0x46dd794e, crc32c(&(0..32).collect::<Vec<u8>>().as_slice()));
    }

    #[test]
    fn rfc3270_decreasing_values() {
        assert_eq!(
            0x113fdb5c,
            crc32c(&(0..32).rev().collect::<Vec<u8>>().as_slice())
        );
    }
}

#[cfg(test)]
#[test]
fn print_message() {
    println!("===============================");
    let mut message: [u8; 4] = [1, 2, 3, 4];
    let message_num: u8 = 2;
    let recipient: u8 = 5;
    let length = message.len();
    // Prep
    frobnicate(&mut message, message_num);
    let message_num_field: [u8; 1] = [message_num];
    let recipient_field: [u8; 1] = [recipient];
    let length_field: [u8; 2] = (length as u16).to_le_bytes();
    // Print:
    fn emit_all(bytes: &[u8], crc: u32) -> u32 {
        for byte in bytes.iter() {
            print!("{:02x}", byte);
        }
        crc32c_update(crc, bytes)
    }
    // With some elements of HDLC:  https://en.wikipedia.org/wiki/High-Level_Data_Link_Control#Asynchronous_framing
    let crc = 0;
    let crc = emit_all(&PREFIX, crc);
    let crc = emit_all(&recipient_field, crc);
    let crc = emit_all(&message_num_field, crc);
    let crc = emit_all(&length_field, crc);
    let crc = emit_all(&message, crc);
    emit_all(&crc.to_le_bytes(), crc);

    println!("");
}
