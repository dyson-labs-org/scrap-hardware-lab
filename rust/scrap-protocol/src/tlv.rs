use crate::ProtocolError;

#[derive(Debug, Clone)]
pub struct TlvRecord {
    pub t: u64,
    pub v: Vec<u8>,
}

pub fn write_bigsize(value: u64, out: &mut Vec<u8>) {
    match value {
        0x00..=0xFC => out.push(value as u8),
        0xFD..=0xFFFF => {
            out.push(0xFD);
            out.extend_from_slice(&(value as u16).to_be_bytes());
        }
        0x1_0000..=0xFFFF_FFFF => {
            out.push(0xFE);
            out.extend_from_slice(&(value as u32).to_be_bytes());
        }
        _ => {
            out.push(0xFF);
            out.extend_from_slice(&value.to_be_bytes());
        }
    }
}

pub fn read_bigsize(input: &[u8]) -> Result<(u64, usize), ProtocolError> {
    if input.is_empty() {
        return Err(ProtocolError::new("bigsize: empty input"));
    }
    let prefix = input[0];
    match prefix {
        0x00..=0xFC => Ok((prefix as u64, 1)),
        0xFD => {
            if input.len() < 3 {
                return Err(ProtocolError::new("bigsize: truncated u16"));
            }
            let value = u16::from_be_bytes([input[1], input[2]]) as u64;
            if value < 0xFD {
                return Err(ProtocolError::new("bigsize: non-canonical u16"));
            }
            Ok((value, 3))
        }
        0xFE => {
            if input.len() < 5 {
                return Err(ProtocolError::new("bigsize: truncated u32"));
            }
            let value = u32::from_be_bytes([input[1], input[2], input[3], input[4]]) as u64;
            if value < 0x1_0000 {
                return Err(ProtocolError::new("bigsize: non-canonical u32"));
            }
            Ok((value, 5))
        }
        0xFF => {
            if input.len() < 9 {
                return Err(ProtocolError::new("bigsize: truncated u64"));
            }
            let value = u64::from_be_bytes([
                input[1], input[2], input[3], input[4], input[5], input[6], input[7], input[8],
            ]);
            if value < 0x1_0000_0000 {
                return Err(ProtocolError::new("bigsize: non-canonical u64"));
            }
            Ok((value, 9))
        }
    }
}

pub fn encode_records(records: &[TlvRecord]) -> Result<Vec<u8>, ProtocolError> {
    let mut out = Vec::new();
    let mut last_type: Option<u64> = None;
    for record in records {
        if let Some(prev) = last_type {
            if record.t < prev {
                return Err(ProtocolError::new("tlv: types must be ascending"));
            }
        }
        last_type = Some(record.t);
        write_bigsize(record.t, &mut out);
        write_bigsize(record.v.len() as u64, &mut out);
        out.extend_from_slice(&record.v);
    }
    Ok(out)
}

pub fn decode_records(bytes: &[u8]) -> Result<Vec<TlvRecord>, ProtocolError> {
    let mut idx = 0usize;
    let mut records = Vec::new();
    let mut last_type: Option<u64> = None;
    while idx < bytes.len() {
        let (t, t_len) = read_bigsize(&bytes[idx..])?;
        idx += t_len;
        let (len, len_len) = read_bigsize(&bytes[idx..])?;
        idx += len_len;
        let len = len as usize;
        if idx + len > bytes.len() {
            return Err(ProtocolError::new("tlv: length exceeds buffer"));
        }
        if let Some(prev) = last_type {
            if t < prev {
                return Err(ProtocolError::new("tlv: types must be ascending"));
            }
        }
        last_type = Some(t);
        let v = bytes[idx..idx + len].to_vec();
        idx += len;
        records.push(TlvRecord { t, v });
    }
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bigsize_roundtrip() {
        let values = [0u64, 1, 252, 253, 65535, 65536, 0x1_0000_0000];
        for value in values {
            let mut buf = Vec::new();
            write_bigsize(value, &mut buf);
            let (decoded, used) = read_bigsize(&buf).expect("decode failed");
            assert_eq!(value, decoded);
            assert_eq!(used, buf.len());
        }
    }

    #[test]
    fn tlv_roundtrip() {
        let records = vec![
            TlvRecord {
                t: 0,
                v: vec![1],
            },
            TlvRecord {
                t: 4,
                v: b"hello".to_vec(),
            },
        ];
        let encoded = encode_records(&records).expect("encode failed");
        let decoded = decode_records(&encoded).expect("decode failed");
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].t, 0);
        assert_eq!(decoded[1].v, b"hello");
    }
}
