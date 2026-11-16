use crate::wallet::Transaction;
use crate::Block;

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
    let mut p =*_pos;
    let n= read_u8(_bytes,&mut p)?;
    
    if n >MAX_STRING{
        return Err(MessageError::TooLarge(n));
    }
    let end =p.checked_add(n as usize ).ok_or(MessageError::Truncated)?;
    let slice = bytes.get(p..end).ok_or(MessageError::Truncated)?;
    let text = String::from_utf8(slice.to_vec()).map_err(|_| MessageError::InvalidUtf8?);
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
    let mut p =*_pos;
    let from =read_string(_bytes,&mut p)?;
    let to=read_string(_bytes,&mut p)?;
    let amount=read_u64(_bytes,&mut p)?;
    let count= read_u64(_bytes, &mut p)?;
    let flag= read_u8(_bytes, &mut p)?;
    let signature = match flag {
    0 => None,
    1 => {
        let end = p.checked_add(64).ok_or(MessageError::Truncated)?;
        let slice = bytes.get(p..end).ok_or(MessageError::Truncated)?;
        let array: [u8; 64] = slice.try_into().map_err(|_| MessageError::Truncated)?;
        p = end;
        Some(ed25519_dalek::Signature::from_bytes(&array))
    }
    _ => return Err(MessageError::InvalidSignature),
};

    
    Ok(Transaction{from, to, amount, signature})
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
    Ok(Block { index, timestamp, data, prev_hash, hash, nonce })
}

pub fn frame_length(header: &[u8]) -> Result<usize, MessageError> {
    if header.len() < HEADER_SIZE {
        return Err(MessageError::Truncated);
    }
    let kind = header[0];
    if !matches!(kind, TYPE_NEW_TRANSACTION | TYPE_NEW_BLOCK | TYPE_GET_CHAIN | TYPE_CHAIN) {
        return Err(MessageError::UnknownType(kind));
    }
    let mut p = 1;
    let n = read_u32(header, &mut p)?;
    if n > MAX_PAYLOAD {
        return Err(MessageError::TooLarge(n));
    }
    Ok(n as usize)
}impl Message {
    pub fn encode(&self) -> Vec<u8> {
        todo!()
    }

    pub fn decode(_bytes: &[u8]) -> Result<Message, MessageError> {
        todo!()
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
        assert_eq!(out, vec![0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 2, b'a', b'b']);
    }
:write_u64}
