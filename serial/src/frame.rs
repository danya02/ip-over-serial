use bytes::{Buf, BytesMut};
use crc::{Crc, CRC_32_BZIP2};
use tokio_util::codec::{Decoder, Encoder};
use tracing::debug;

pub type MacAddress = [u8; 6];

#[derive(Clone, PartialEq, Debug)]
pub struct SerialEthFrame {
    destination: MacAddress,
    source: MacAddress,
    vlan_tag: u32,
    payload: Vec<u8>,
}

impl SerialEthFrame {
    pub fn new(
        destination: &MacAddress,
        source: &MacAddress,
        vlan_tag: u32,
        payload: &[u8],
    ) -> Self {
        Self {
            destination: *destination,
            source: *source,
            vlan_tag,
            payload: payload.to_vec(),
        }
    }
}

pub struct SerialEthFrameCodec {}

impl Encoder<SerialEthFrame> for SerialEthFrameCodec {
    type Error = std::io::Error;
    fn encode(&mut self, item: SerialEthFrame, dst: &mut BytesMut) -> Result<(), Self::Error> {
        if item.payload.len() > u16::MAX as usize {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Frame payload too big",
            ));
        }

        dst.reserve(
            7 + 1 + // Preamble, start frame delimiter
            6 + 6 + 4 + // dest, src, vlan tag
            2 + item.payload.len() + 4 +  // length, payload, frame crc
            12, // interpacket gap
        );

        let algo = Crc::<u32>::new(&CRC_32_BZIP2);

        dst.extend_from_slice(&[0b10101010; 7]); // preamble
        dst.extend_from_slice(&[0b10101011]); // sfd
        let begin_length = dst.len();
        dst.extend_from_slice(&item.destination);
        dst.extend_from_slice(&item.source);
        dst.extend_from_slice(&item.vlan_tag.to_be_bytes());
        let len = item.payload.len() as u16;
        dst.extend_from_slice(&len.to_be_bytes());
        dst.extend_from_slice(&item.payload);
        let end_length = dst.len();
        let frame_data = &dst[begin_length..end_length];
        let crc = algo.checksum(frame_data);
        dst.extend_from_slice(&crc.to_be_bytes());

        dst.extend_from_slice(&[0; 12]);

        Ok(())
    }
}

impl Decoder for SerialEthFrameCodec {
    type Item = SerialEthFrame;

    type Error = std::io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // Look for a sequence of 3 0b10101010 bytes followed by a 0b10101011 byte.
        let pattern = [0b10101010, 0b10101010, 0b10101010, 0b10101011];
        let mut frame_start = None;
        if src.len() < 4 {
            debug!("Less than 4 bytes in buffer");
            return Ok(None);
        }

        for start_index in 0..src.len() - 4 {
            if src[start_index..start_index + 4] == pattern {
                frame_start = Some(start_index + 4); // frame starts after the end of the 0b10101011.
                break;
            }
        }
        if frame_start.is_none() {
            debug!("Could not find start of frame");
            return Ok(None);
        }
        let frame_start = frame_start.unwrap();
        debug!("Found start of frame at {frame_start}");

        // Remove bytes before the start of the 4-byte sequence.
        src.advance(frame_start - 4);
        let true_src = src;
        let src = &true_src[4..];

        // Make sure that the buffer contains 6+6+4+2 bytes, and get the last two bytes.
        if src.len() < 6 + 6 + 4 + 2 {
            return Ok(None);
        }
        let length = u16::from_be_bytes([src[6 + 6 + 4], src[6 + 6 + 4 + 1]]);
        debug!("Frame payload has length of {length}");
        // Make sure that all of the payload, and the CRC, has been received.
        if src.len() < (6 + 6 + 4 + 2 + length + 4) as usize {
            debug!(
                "Not all buffer has been received (only {} available)",
                src.len()
            );
            return Ok(None);
        }

        let src_addr: MacAddress = src[0..6].try_into().unwrap();
        let dst_addr: MacAddress = src[6..12].try_into().unwrap();
        let vlan_tag: u32 = u32::from_be_bytes(src[12..16].try_into().unwrap());
        let payload = &src[16 + 2..16 + 2 + length as usize];

        let algo = Crc::<u32>::new(&CRC_32_BZIP2);
        let expected_crc = algo.checksum(&src[0..16 + 2 + length as usize]);
        let got_crc = u32::from_be_bytes(
            src[16 + 2 + length as usize..16 + 2 + length as usize + 4]
                .try_into()
                .unwrap(),
        );
        if expected_crc != got_crc {
            // If CRC is wrong, we drop the frame: advance the buffer to delete the start of frame,
            // that way we won't see this frame again.
            true_src.advance(4);

            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Frame CRC was {got_crc}, but computed CRC was {expected_crc}"),
            ));
        }

        // Construct the result
        let output = SerialEthFrame {
            destination: dst_addr,
            source: src_addr,
            vlan_tag,
            payload: payload.to_vec(),
        };

        // Advance the buffer
        true_src.advance(4 + 16 + 2 + length as usize + 4);

        Ok(Some(output))
    }
}
