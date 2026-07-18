#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum MessageType {
    FullClientRequest = 0x1,
    AudioOnlyRequest = 0x2,
    FullServerResponse = 0x9,
    ErrorMessage = 0xf,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Flags {
    PositiveSequence = 0x1,
    LastPacket = 0x2,
    NegativeSequence = 0x3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub(crate) enum Serialization {
    None = 0x0,
    Json = 0x1,
}

#[derive(Debug)]
pub(crate) struct ParsedFrame {
    pub(crate) message_type: Option<MessageType>,
    pub(crate) flags: u8,
    pub(crate) sequence: Option<i32>,
    pub(crate) error_code: Option<u32>,
    pub(crate) payload: Vec<u8>,
}

impl ParsedFrame {
    pub(crate) fn is_final(&self) -> bool {
        self.flags == Flags::LastPacket as u8
            || self.flags == Flags::NegativeSequence as u8
            || self.sequence.is_some_and(|sequence| sequence < 0)
    }
}

pub(crate) fn build(
    message_type: MessageType,
    flags: Flags,
    serialization: Serialization,
    payload: &[u8],
    sequence: Option<i32>,
) -> Vec<u8> {
    let mut frame = Vec::with_capacity(payload.len() + 12);
    frame.push(0x11);
    frame.push(((message_type as u8) << 4) | flags as u8);
    frame.push((serialization as u8) << 4);
    frame.push(0x00);
    if matches!(flags, Flags::PositiveSequence | Flags::NegativeSequence)
        && let Some(sequence) = sequence
    {
        frame.extend_from_slice(&sequence.to_be_bytes());
    }
    frame.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    frame.extend_from_slice(payload);
    frame
}

pub(crate) fn parse(data: &[u8]) -> Option<ParsedFrame> {
    if data.len() < 8 || data[2] & 0x0f != 0 {
        return None;
    }
    let header_size = ((data[0] & 0x0f) as usize) * 4;
    if header_size < 4 || data.len() < header_size + 4 {
        return None;
    }
    let message_type = match data[1] >> 4 {
        0x1 => Some(MessageType::FullClientRequest),
        0x2 => Some(MessageType::AudioOnlyRequest),
        0x9 => Some(MessageType::FullServerResponse),
        0xf => Some(MessageType::ErrorMessage),
        _ => None,
    };
    let flags = data[1] & 0x0f;
    let mut offset = header_size;
    let sequence = if matches!(flags, 0x1 | 0x3) {
        let value = read_u32(data, offset)? as i32;
        offset += 4;
        Some(value)
    } else {
        None
    };
    if message_type == Some(MessageType::ErrorMessage) {
        let error_code = read_u32(data, offset)?;
        let payload_size = read_u32(data, offset + 4)? as usize;
        offset += 8;
        return Some(ParsedFrame {
            message_type,
            flags,
            sequence,
            error_code: Some(error_code),
            payload: data.get(offset..offset + payload_size)?.to_vec(),
        });
    }
    let payload_size = read_u32(data, offset)? as usize;
    offset += 4;
    Some(ParsedFrame {
        message_type,
        flags,
        sequence,
        error_code: None,
        payload: data.get(offset..offset + payload_size)?.to_vec(),
    })
}

fn read_u32(data: &[u8], offset: usize) -> Option<u32> {
    Some(u32::from_be_bytes(
        data.get(offset..offset + 4)?.try_into().ok()?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_sequence_and_payload() {
        let frame = build(
            MessageType::FullClientRequest,
            Flags::PositiveSequence,
            Serialization::Json,
            b"hello",
            Some(1),
        );
        let parsed = parse(&frame).unwrap();
        assert_eq!(parsed.message_type, Some(MessageType::FullClientRequest));
        assert_eq!(parsed.sequence, Some(1));
        assert_eq!(parsed.payload, b"hello");
        assert!(!parsed.is_final());
    }

    #[test]
    fn negative_sequence_marks_final_frame() {
        let frame = build(
            MessageType::AudioOnlyRequest,
            Flags::NegativeSequence,
            Serialization::None,
            &[],
            Some(-3),
        );
        assert!(parse(&frame).unwrap().is_final());
    }
}
