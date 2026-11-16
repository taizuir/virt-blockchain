use crate::Block;
use crate::wallet::Transaction;

pub const HEADER_SIZE: usize = 5;
pub const MAX_PAYLOAD: u32 = 1_048_576;
pub const MAX_STRING: u32 = 256;
pub const MAX_BLOCKS: u32 = 10_000;
pub const MAX_TX_PER_BLOCK: u32 = 10_000;

pub const TYPE_NEW_TRANSACTION: u8 = 0x01;
pub const TYPE_NEW_BLOCK: u8 = 0x02;
pub const TYPE_GET_CHAIN: u8 = 0x03;
pub const TYPE_CHAIN: u8 = 0x04;

#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    NewTransaction(Transaction),
    NewBlock(Block),
    GetChain,
    Chain(Vec<Block>),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum MessageError {
    Truncated,
    UnknownType(u8),
    TooLarge(u32),
    LengthMismatch,
    TrailingBytes,
    InvalidUtf8,
    InvalidSignature,
    StringTooLong(u32),
    TooManyItems(u32),
}

fn write_u8(out: &mut Vec<u8>, v: u8) {
    out.push(v);
}

fn write_u32(out: &mut Vec<u8>, v: u32) {
    out.extend_from_slice(&v.to_be_bytes());
}

fn write_u64(out: &mut Vec<u8>, v: u64) {
    out.extend_from_slice(&v.to_be_bytes());
}

fn write_string(out: &mut Vec<u8>, s: &str) {
    write_u32(out, s.len() as u32);
    out.extend_from_slice(s.as_bytes());
}

fn read_u8(bytes: &[u8], pos: &mut usize) -> Result<u8, MessageError> {
    let b = *bytes.get(*pos).ok_or(MessageError::Truncated)?;
    *pos += 1;
    Ok(b)
}

fn read_u32(bytes: &[u8], pos: &mut usize) -> Result<u32, MessageError> {
    let end = pos.checked_add(4).ok_or(MessageError::Truncated)?;
    let slice = bytes.get(*pos..end).ok_or(MessageError::Truncated)?;
    let array: [u8; 4] = slice.try_into().map_err(|_| MessageError::Truncated)?;
    *pos = end;
    Ok(u32::from_be_bytes(array))
}

fn read_u64(bytes: &[u8], pos: &mut usize) -> Result<u64, MessageError> {
    let end = pos.checked_add(8).ok_or(MessageError::Truncated)?;
    let slice = bytes.get(*pos..end).ok_or(MessageError::Truncated)?;
    let array: [u8; 8] = slice.try_into().map_err(|_| MessageError::Truncated)?;
    *pos = end;
    Ok(u64::from_be_bytes(array))
}

fn read_string(_bytes: &[u8], _pos: &mut usize) -> Result<String, MessageError> {
    let mut p = *_pos;
    let n = read_u8(_bytes, &mut p)?;

    if n > MAX_STRING {
        return Err(MessageError::TooLarge(n));
    }
    let end = p.checked_add(n as usize).ok_or(MessageError::Truncated)?;
    let slice = bytes.get(p..end).ok_or(MessageError::Truncated)?;
    let text = String::from_utf8(slice.to_vec()).map_err(|_| MessageError::InvalidUtf8)?;
    *_pos = end;

    Ok(text)
}

fn write_transaction(out: &mut Vec<u8>, t: &Transaction) {
    write_string(out, &t.from);
    write_string(out, &t.to);
    write_u64(out, t.amount);
    write_u64(out, t.count);
    match &t.signature {
        None => write_u8(out, 0),
        Some(sig) => {
            write_u8(out, 1);
            out.extend_from_slice(&sig.to_bytes());
        }
    }
}

fn read_transaction(_bytes: &[u8], _pos: &mut usize) -> Result<Transaction, MessageError> {
    let mut p = *_pos;
    let from = read_string(_bytes, &mut p)?;
    let to = read_string(_bytes, &mut p)?;
    let amount = read_u64(_bytes, &mut p)?;
    let count = read_u64(_bytes, &mut p)?;
    let flag = read_u8(_bytes, &mut p)?;
    let signature = match flag {
        0 => None,
        1 => {
            let end = p.checked_add(64).ok_or(MessageError::Truncated)?;
            let slice = _bytes.get(p..end).ok_or(MessageError::Truncated)?;
            let array: [u8; 64] = slice.try_into().map_err(|_| MessageError::Truncated)?;
            p = end;
            Some(ed25519_dalek::Signature::from_bytes(&array))
        }
        _ => return Err(MessageError::InvalidSignature),
    };

    Ok(Transaction {
        from,
        to,
        amount,
        signature,
    })
}

fn write_block(out: &mut Vec<u8>, b: &Block) {
    write_u64(out, b.index);
    write_u64(out, b.timestamp);
    write_string(out, &b.prev_hash);
    write_string(out, &b.hash);
    write_u64(out, b.nonce);
    write_u32(out, b.data.len() as u32);
    for t in &b.data {
        write_transaction(out, t);
    }
}

fn read_block(bytes: &[u8], pos: &mut usize) -> Result<Block, MessageError> {
    let mut p = *pos;
    let index = read_u64(bytes, &mut p)?;
    let timestamp = read_u64(bytes, &mut p)?;
    let prev_hash = read_string(bytes, &mut p)?;
    let hash = read_string(bytes, &mut p)?;
    let nonce = read_u64(bytes, &mut p)?;
    let count = read_u32(bytes, &mut p)?;
    if count > MAX_TX_PER_BLOCK {
        return Err(MessageError::TooManyItems(count));
    }
    let mut data = Vec::new();
    for _ in 0..count {
        data.push(read_transaction(bytes, &mut p)?);
    }
    *pos = p;
    Ok(Block {
        index,
        timestamp,
        data,
        prev_hash,
        hash,
        nonce,
    })
}

pub fn frame_length(header: &[u8]) -> Result<usize, MessageError> {
    if header.len() < HEADER_SIZE {
        return Err(MessageError::Truncated);
    }
    let kind = header[0];
    if !matches!(
        kind,
        TYPE_NEW_TRANSACTION | TYPE_NEW_BLOCK | TYPE_GET_CHAIN | TYPE_CHAIN
    ) {
        return Err(MessageError::UnknownType(kind));
    }
    let mut p = 1;
    let n = read_u32(header, &mut p)?;
    if n > MAX_PAYLOAD {
        return Err(MessageError::TooLarge(n));
    }
    Ok(n as usize)
}
impl Message {
    pub fn encode(&self) -> Vec<u8> {
        let mut payload = Vec::new();
        let kind = match self {
            Message::NewTransaction(t) => {
                write_transaction(&mut payload, t);
                TYPE_NEW_TRANSACTION
            }
            Message::NewBlock(b) => {
                write_block(&mut payload, b);
                TYPE_NEW_BLOCK
            }
            Message::GetChain => TYPE_GET_CHAIN,
            Message::Chain(blocks) => {
                write_u32(&mut payload, blocks.len() as u32);
                for b in blocks {
                    write_block(&mut payload, b);
                }
                TYPE_CHAIN
            }
        };
        let mut out = Vec::with_capacity(HEADER_SIZE + payload.len());
        write_u8(&mut out, kind);
        write_u32(&mut out, payload.len() as u32);
        out.extend_from_slice(&payload);
        out
    }

    pub fn decode(bytes: &[u8]) -> Result<Message, MessageError> {
        let n = frame_length(bytes)?;
        if bytes.len() != HEADER_SIZE + n {
            return Err(MessageError::LengthMismatch);
        }
        let mut pos = HEADER_SIZE;
        let message = match bytes[0] {
            TYPE_NEW_TRANSACTION => Message::NewTransaction(read_transaction(bytes, &mut pos)?),
            TYPE_NEW_BLOCK => Message::NewBlock(read_block(bytes, &mut pos)?),
            TYPE_GET_CHAIN => Message::GetChain,
            TYPE_CHAIN => {
                let count = read_u32(bytes, &mut pos)?;
                if count > MAX_BLOCKS {
                    return Err(MessageError::TooManyItems(count));
                }
                let mut blocks = Vec::new();
                for _ in 0..count {
                    blocks.push(read_block(bytes, &mut pos)?);
                }
                Message::Chain(blocks)
            }
            other => return Err(MessageError::UnknownType(other)),
        };
        if pos != bytes.len() {
            return Err(MessageError::TrailingBytes);
        }
        Ok(message)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_u8_u32_u64_advance_the_position() {
        let bytes = [7, 0, 0, 0, 9, 0, 0, 0, 0, 0, 0, 1, 0];
        let mut pos = 0;
        assert_eq!(read_u8(&bytes, &mut pos), Ok(7));
        assert_eq!(read_u32(&bytes, &mut pos), Ok(9));
        assert_eq!(read_u64(&bytes, &mut pos), Ok(256));
        assert_eq!(pos, 13);
    }

    #[test]
    fn readers_return_truncated_instead_of_panicking() {
        let mut pos = 0;
        assert_eq!(read_u8(&[], &mut pos), Err(MessageError::Truncated));
        assert_eq!(read_u32(&[1, 2, 3], &mut pos), Err(MessageError::Truncated));
        assert_eq!(read_u64(&[0; 7], &mut pos), Err(MessageError::Truncated));
        assert_eq!(pos, 0);
    }

    #[test]
    fn write_helpers_are_big_endian() {
        let mut out = vec![];
        write_u32(&mut out, 1);
        write_u64(&mut out, 2);
        write_string(&mut out, "ab");
        assert_eq!(
            out,
            vec![0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 2, b'a', b'b']
        );
    }

    #[test]
    fn read_string_reads_two_strings_in_a_row() {
        let bytes = [0, 0, 0, 2, 0x61, 0x61, 0, 0, 0, 3, 0x62, 0x62, 0x62];
        let mut pos = 0;
        assert_eq!(read_string(&bytes, &mut pos), Ok("aa".to_string()));
        assert_eq!(pos, 6);
        assert_eq!(read_string(&bytes, &mut pos), Ok("bbb".to_string()));
        assert_eq!(pos, 13);
    }

    #[test]
    fn read_string_errors_leave_pos_unchanged() {
        let mut pos = 0;
        assert_eq!(read_string(&[0, 0], &mut pos), Err(MessageError::Truncated));
        assert_eq!(read_string(&[], &mut pos), Err(MessageError::Truncated));
        assert_eq!(
            read_string(&[0, 0, 0, 5, 0x68, 0x65], &mut pos),
            Err(MessageError::Truncated)
        );
        assert_eq!(
            read_string(&[0, 0, 1, 0x2c], &mut pos),
            Err(MessageError::StringTooLong(300))
        );
        assert_eq!(
            read_string(&[0, 0, 0, 2, 0xff, 0xfe], &mut pos),
            Err(MessageError::InvalidUtf8)
        );
        assert_eq!(
            read_string(&[0xff, 0xff, 0xff, 0xff], &mut pos),
            Err(MessageError::StringTooLong(u32::MAX))
        );
        assert_eq!(pos, 0);
    }

    #[test]
    fn read_string_accepts_exactly_the_maximum() {
        let mut bytes = vec![0, 0, 1, 0];
        bytes.extend(vec![b'a'; 256]);
        let mut pos = 0;
        assert_eq!(read_string(&bytes, &mut pos).map(|s| s.len()), Ok(256));
        assert_eq!(pos, 260);
    }

    #[test]
    fn read_string_roundtrips_with_write_string() {
        let mut out = vec![];
        write_string(&mut out, "hello");
        let mut pos = 0;
        assert_eq!(read_string(&out, &mut pos), Ok("hello".to_string()));
        assert_eq!(pos, out.len());
    }

    #[test]
    fn write_transaction_unsigned_layout() {
        let mut t = Transaction::new("aa".to_string(), "bb".to_string(), 5);
        t.count = 1;
        let mut out = vec![];
        write_transaction(&mut out, &t);
        assert_eq!(out.len(), 29);
        assert_eq!(
            out,
            vec![
                0, 0, 0, 2, 0x61, 0x61, 0, 0, 0, 2, 0x62, 0x62, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0,
                0, 0, 0, 1, 0,
            ]
        );
    }

    #[test]
    fn write_transaction_signed_is_217_bytes() {
        use crate::wallet::Wallet;
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut t = Transaction::new(
            hex::encode(a.public_key.to_bytes()),
            hex::encode(b.public_key.to_bytes()),
            5,
        );
        t.sign(&a);
        let mut out = vec![];
        write_transaction(&mut out, &t);
        assert_eq!(out.len(), 217);
        assert_eq!(out[216 - 64], 1);
    }

    #[test]
    fn read_transaction_roundtrip_signed_and_unsigned() {
        use crate::wallet::Wallet;
        let (a, b) = (Wallet::new(), Wallet::new());
        let mut signed = Transaction::new(
            hex::encode(a.public_key.to_bytes()),
            hex::encode(b.public_key.to_bytes()),
            9,
        );
        signed.count = 3;
        signed.sign(&a);
        let mut unsigned = Transaction::new("aa".to_string(), "bb".to_string(), 5);
        unsigned.count = 1;
        for t in [signed, unsigned] {
            let mut out = vec![];
            write_transaction(&mut out, &t);
            let mut pos = 0;
            assert_eq!(read_transaction(&out, &mut pos), Ok(t));
            assert_eq!(pos, out.len());
        }
    }

    #[test]
    fn read_transaction_bad_flag_and_truncation() {
        let mut t = Transaction::new("aa".to_string(), "bb".to_string(), 5);
        t.count = 1;
        let mut out = vec![];
        write_transaction(&mut out, &t);
        let last = out.len() - 1;
        out[last] = 2;
        let mut pos = 0;
        assert_eq!(
            read_transaction(&out, &mut pos),
            Err(MessageError::InvalidSignature)
        );
        assert_eq!(pos, 0);
        out[last] = 1; // announces a signature that is not there
        assert_eq!(
            read_transaction(&out, &mut pos),
            Err(MessageError::Truncated)
        );
        for cut in 0..last {
            assert!(read_transaction(&out[..cut], &mut pos).is_err());
        }
        assert_eq!(pos, 0);
    }

    #[test]
    fn write_block_layout_without_transactions() {
        let b = Block {
            index: 1,
            timestamp: 2,
            data: vec![],
            prev_hash: "0".to_string(),
            hash: "ab".to_string(),
            nonce: 3,
        };
        let mut out = vec![];
        write_block(&mut out, &b);
        assert_eq!(
            out,
            vec![
                0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 1, 0x30, 0, 0, 0, 2, 0x61,
                0x62, 0, 0, 0, 0, 0, 0, 0, 3, 0, 0, 0, 0,
            ]
        );
        assert_eq!(out.len(), 39);
    }

    #[test]
    fn write_block_with_one_transaction_has_the_right_size() {
        let mut t = Transaction::new("aa".to_string(), "bb".to_string(), 5);
        t.count = 1;
        let b = Block {
            index: 1,
            timestamp: 2,
            data: vec![t],
            prev_hash: "0".to_string(),
            hash: "ab".to_string(),
            nonce: 3,
        };
        let mut out = vec![];
        write_block(&mut out, &b);
        assert_eq!(out.len(), 39 + 29);
        assert_eq!(&out[35..39], &[0, 0, 0, 1]);
    }

    fn sample_block(with_tx: bool) -> Block {
        let mut data = vec![];
        if with_tx {
            let mut t = Transaction::new("aa".to_string(), "bb".to_string(), 5);
            t.count = 1;
            data.push(t);
        }
        Block {
            index: 1,
            timestamp: 2,
            data,
            prev_hash: "0".to_string(),
            hash: "ab".to_string(),
            nonce: 3,
        }
    }

    #[test]
    fn read_block_roundtrip() {
        for with_tx in [false, true] {
            let b = sample_block(with_tx);
            let mut out = vec![];
            write_block(&mut out, &b);
            let mut pos = 0;
            assert_eq!(read_block(&out, &mut pos), Ok(b));
            assert_eq!(pos, out.len());
        }
    }

    #[test]
    fn read_block_every_truncation_fails_and_keeps_pos() {
        let mut out = vec![];
        write_block(&mut out, &sample_block(true));
        let mut pos = 0;
        for cut in 0..out.len() {
            assert!(read_block(&out[..cut], &mut pos).is_err(), "cut {cut}");
        }
        assert_eq!(pos, 0);
    }

    #[test]
    fn read_block_rejects_huge_transaction_count_without_allocating() {
        let mut out = vec![];
        write_block(&mut out, &sample_block(false));
        let n = out.len();
        out[n - 4..].copy_from_slice(&u32::MAX.to_be_bytes());
        let mut pos = 0;
        assert_eq!(
            read_block(&out, &mut pos),
            Err(MessageError::TooManyItems(u32::MAX))
        );
        out[n - 4..].copy_from_slice(&(MAX_TX_PER_BLOCK + 1).to_be_bytes());
        assert_eq!(
            read_block(&out, &mut pos),
            Err(MessageError::TooManyItems(MAX_TX_PER_BLOCK + 1))
        );
        assert_eq!(pos, 0);
    }

    #[test]
    fn frame_length_valid_headers() {
        assert_eq!(frame_length(&[3, 0, 0, 0, 0]), Ok(0));
        assert_eq!(frame_length(&[1, 0, 0, 0, 29]), Ok(29));
        assert_eq!(frame_length(&[4, 0, 0x10, 0, 0]), Ok(1_048_576));
        assert_eq!(frame_length(&[2, 0, 0, 0, 5, 9, 9]), Ok(5));
    }

    #[test]
    fn frame_length_errors() {
        assert_eq!(frame_length(&[]), Err(MessageError::Truncated));
        assert_eq!(frame_length(&[3, 0, 0, 0]), Err(MessageError::Truncated));
        assert_eq!(
            frame_length(&[9, 0, 0, 0, 0]),
            Err(MessageError::UnknownType(9))
        );
        assert_eq!(
            frame_length(&[0, 0, 0, 0, 0]),
            Err(MessageError::UnknownType(0))
        );
        assert_eq!(
            frame_length(&[1, 0, 0x10, 0, 1]),
            Err(MessageError::TooLarge(1_048_577))
        );
        assert_eq!(
            frame_length(&[1, 0xff, 0xff, 0xff, 0xff]),
            Err(MessageError::TooLarge(u32::MAX))
        );
    }

    fn sample_tx(signed: bool) -> Transaction {
        let mut t = Transaction::new("aa".to_string(), "bb".to_string(), 5);
        t.count = 1;
        if signed {
            t.signature = Some(ed25519_dalek::Signature::from_bytes(&[0x11; 64]));
        }
        t
    }

    fn chain_message() -> Message {
        let genesis = Block {
            index: 0,
            timestamp: 0,
            data: vec![],
            prev_hash: "0".to_string(),
            hash: "00ab".to_string(),
            nonce: 7,
        };
        let mut t = Transaction::new("aa".to_string(), "bb".to_string(), 5);
        t.signature = Some(ed25519_dalek::Signature::from_bytes(&[0x11; 64]));
        let second = Block {
            index: 1,
            timestamp: 5,
            data: vec![t],
            prev_hash: "00ab".to_string(),
            hash: "00cd".to_string(),
            nonce: 9,
        };
        Message::Chain(vec![genesis, second])
    }

    #[test]
    fn get_chain_roundtrip_and_five_bytes() {
        let bytes = Message::GetChain.encode();
        assert_eq!(bytes, vec![3, 0, 0, 0, 0]);
        assert_eq!(Message::decode(&bytes), Ok(Message::GetChain));
    }

    #[test]
    fn transaction_message_roundtrip_and_exact_bytes() {
        let m = Message::NewTransaction(sample_tx(false));
        let bytes = m.encode();
        assert_eq!(bytes.len(), 34);
        assert_eq!(&bytes[..5], &[1, 0, 0, 0, 29]);
        assert_eq!(Message::decode(&bytes), Ok(m));
        let m = Message::NewTransaction(sample_tx(true));
        assert_eq!(Message::decode(&m.encode()), Ok(m));
    }

    #[test]
    fn block_message_roundtrip() {
        for with_tx in [false, true] {
            let m = Message::NewBlock(sample_block(with_tx));
            assert_eq!(Message::decode(&m.encode()), Ok(m));
        }
        assert_eq!(Message::NewBlock(sample_block(false)).encode().len(), 44);
    }

    #[test]
    fn chain_message_roundtrip_and_187_bytes() {
        let m = chain_message();
        let bytes = m.encode();
        assert_eq!(bytes.len(), 187);
        assert_eq!(&bytes[..5], &[4, 0, 0, 0, 182]);
        assert_eq!(Message::decode(&bytes), Ok(m));
        let empty = Message::Chain(vec![]);
        assert_eq!(empty.encode(), vec![4, 0, 0, 0, 4, 0, 0, 0, 0]);
        assert_eq!(Message::decode(&empty.encode()), Ok(empty));
    }

    #[test]
    fn decode_header_errors() {
        assert_eq!(Message::decode(&[]), Err(MessageError::Truncated));
        assert_eq!(Message::decode(&[3, 0, 0]), Err(MessageError::Truncated));
        assert_eq!(
            Message::decode(&[9, 0, 0, 0, 0]),
            Err(MessageError::UnknownType(9))
        );
        assert_eq!(
            Message::decode(&[1, 0xff, 0xff, 0xff, 0xff]),
            Err(MessageError::TooLarge(u32::MAX))
        );
    }

    #[test]
    fn decode_length_mismatch_both_ways() {
        let mut bytes = chain_message().encode();
        let mut longer = bytes.clone();
        longer.extend_from_slice(&[0, 0, 0]);
        assert_eq!(Message::decode(&longer), Err(MessageError::LengthMismatch));
        bytes.truncate(bytes.len() - 3);
        assert_eq!(Message::decode(&bytes), Err(MessageError::LengthMismatch));
    }

    #[test]
    fn decode_trailing_bytes_inside_the_announced_length() {
        assert_eq!(
            Message::decode(&[3, 0, 0, 0, 3, 1, 2, 3]),
            Err(MessageError::TrailingBytes)
        );
        let mut bytes = Message::NewTransaction(sample_tx(false)).encode();
        bytes.extend_from_slice(&[9, 9, 9]);
        let n = (bytes.len() - 5) as u32;
        bytes[1..5].copy_from_slice(&n.to_be_bytes());
        assert_eq!(Message::decode(&bytes), Err(MessageError::TrailingBytes));
    }

    #[test]
    fn decode_invalid_utf8_flag_string_length_and_counts() {
        let base = chain_message().encode();

        let mut b = base.clone();
        b[98] = 0xff;
        b[99] = 0xfe;
        assert_eq!(Message::decode(&b), Err(MessageError::InvalidUtf8));

        let mut b = base.clone();
        b[122] = 2;
        assert_eq!(Message::decode(&b), Err(MessageError::InvalidSignature));

        let mut b = base.clone();
        b[94..98].copy_from_slice(&300u32.to_be_bytes());
        assert_eq!(Message::decode(&b), Err(MessageError::StringTooLong(300)));

        let mut b = base.clone();
        b[5..9].copy_from_slice(&(MAX_BLOCKS + 1).to_be_bytes());
        assert_eq!(
            Message::decode(&b),
            Err(MessageError::TooManyItems(MAX_BLOCKS + 1))
        );

        let mut b = base.clone();
        b[5..9].copy_from_slice(&u32::MAX.to_be_bytes());
        assert_eq!(
            Message::decode(&b),
            Err(MessageError::TooManyItems(u32::MAX))
        );

        let mut b = base.clone();
        b[90..94].copy_from_slice(&(MAX_TX_PER_BLOCK + 1).to_be_bytes());
        assert_eq!(
            Message::decode(&b),
            Err(MessageError::TooManyItems(MAX_TX_PER_BLOCK + 1))
        );
    }

    #[test]
    fn declared_counts_larger_than_the_data_are_truncated() {
        let mut b = chain_message().encode();
        b[5..9].copy_from_slice(&5u32.to_be_bytes());
        assert_eq!(Message::decode(&b), Err(MessageError::Truncated));
    }

    #[test]
    fn every_truncation_of_a_valid_frame_fails_cleanly() {
        for m in [
            Message::GetChain,
            Message::NewTransaction(sample_tx(true)),
            Message::NewBlock(sample_block(true)),
            chain_message(),
        ] {
            let bytes = m.encode();
            for cut in 0..bytes.len() {
                assert!(Message::decode(&bytes[..cut]).is_err(), "cut {cut}");
            }
        }
    }

    #[test]
    fn every_single_byte_corruption_never_panics() {
        let bytes = chain_message().encode();
        for i in 0..bytes.len() {
            for v in [0u8, 1, 2, 0x7f, 0x80, 0xff] {
                let mut b = bytes.clone();
                b[i] = v;
                let _ = Message::decode(&b);
            }
        }
    }

    #[test]
    fn random_bytes_never_panic() {
        let mut seed: u64 = 0x1234_5678_9abc_def0;
        let mut next = || {
            seed = seed
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (seed >> 33) as u32
        };
        for _ in 0..20_000 {
            let len = (next() % 300) as usize;
            let mut b: Vec<u8> = (0..len).map(|_| next() as u8).collect();
            if !b.is_empty() && next() % 2 == 0 {
                b[0] = 1 + (next() % 4) as u8;
            }
            if b.len() >= 5 && next() % 2 == 0 {
                let n = (b.len() - 5) as u32;
                b[1..5].copy_from_slice(&n.to_be_bytes());
            }
            let _ = Message::decode(&b);
        }
    }
}
