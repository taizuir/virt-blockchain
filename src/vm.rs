use std::collections::BTreeMap;

use crate::vm::VMErrors::{InvalidRole, ProgramTooLong};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VMErrors {
    ProgramTooLong,
    UnknownOpcode(u8),
    TruncatedOperand,
    InvalidRole(u8),
    InvalidJumpTarget(u32),
    StackUnderflow,
    StackOverflow,
    ArithmeticOverflow,
    DivisionByZero,
    MemoryOutOfBounds(usize),
    OutOfGas,
    Reverted,
}
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Instructions {
    Add,
    Push(u64),
    Subtraction,
    Swap,
    Pop,
    Double,
    Product,
    Divide,
    Greater,
    Lower,
    Equal,
    Load(u8),
    Store(u8),
    Jump(u32),
    JumpIfZero(u32),
    Halt,
    Revert,
    PushAmount,
    LoadBalance(u8),
    StoreBalance(u8),
}
pub struct Vm {
    pile: Vec<u64>,
    memory: [u64; 16],
    gas: u64,
    context: Vec<(String, String, u64)>,
    persistentstate: BTreeMap<String, u64>,
}
impl Vm {
    pub fn new(context: Vec<(String, String, u64)>) -> Vm {
        let pile = vec![];
        let memory = [0; 16];
        let gas = 10000;
        let persistentstate = BTreeMap::new();
        Vm {
            pile,
            memory,
            gas,
            context,
            persistentstate,
        }
    }
    fn pop(&mut self) -> Result<u64, VMErrors> {
        self.pile.pop().ok_or(VMErrors::StackUnderflow)
    }
    fn push(&mut self, elmt: u64) -> Result<(), VMErrors> {
        if self.pile.len() >= 256 {
            return Err(VMErrors::StackOverflow);
        } else {
            self.pile.push(elmt);
            Ok(())
        }
    }
}
fn read_u8(bytes: &[u8]) -> Result<u8, VMErrors> {
    bytes.get(1).copied().ok_or(VMErrors::TruncatedOperand)
}

fn read_u32(bytes: &[u8]) -> Result<u32, VMErrors> {
    let raw: [u8; 4] = bytes
        .get(1..5)
        .ok_or(VMErrors::TruncatedOperand)?
        .try_into()
        .map_err(|_| VMErrors::TruncatedOperand)?;
    Ok(u32::from_be_bytes(raw))
}

fn read_u64(bytes: &[u8]) -> Result<u64, VMErrors> {
    let raw: [u8; 8] = bytes
        .get(1..9)
        .ok_or(VMErrors::TruncatedOperand)?
        .try_into()
        .map_err(|_| VMErrors::TruncatedOperand)?;
    Ok(u64::from_be_bytes(raw))
}
impl Instructions {
    pub fn encode(&self, bytes: &mut Vec<u8>) {
        match self {
            Instructions::Halt => bytes.push(0x00),
            Instructions::Revert => bytes.push(0x01),
            Instructions::Push(value) => {
                bytes.push(0x10);
                bytes.extend_from_slice(&value.to_be_bytes());
            }
            Instructions::Pop => bytes.push(0x11),
            Instructions::Double => bytes.push(0x12),
            Instructions::Swap => bytes.push(0x13),
            Instructions::Add => bytes.push(0x20),
            Instructions::Subtraction => bytes.push(0x21),
            Instructions::Product => bytes.push(0x22),
            Instructions::Divide => bytes.push(0x23),
            Instructions::Equal => bytes.push(0x30),
            Instructions::Lower => bytes.push(0x31),
            Instructions::Greater => bytes.push(0x32),
            Instructions::Load(cell) => {
                bytes.push(0x40);
                bytes.extend_from_slice(&cell.to_be_bytes());
            }
            Instructions::Store(cell) => {
                bytes.push(0x41);
                bytes.extend_from_slice(&cell.to_be_bytes());
            }
            Instructions::Jump(target) => {
                bytes.push(0x50);
                bytes.extend_from_slice(&target.to_be_bytes());
            }
            Instructions::JumpIfZero(target) => {
                bytes.push(0x51);
                bytes.extend_from_slice(&target.to_be_bytes());
            }
            Instructions::PushAmount => bytes.push(0x60),
            Instructions::LoadBalance(role) => {
                bytes.push(0x61);
                bytes.extend_from_slice(&role.to_be_bytes());
            }
            Instructions::StoreBalance(role) => {
                bytes.push(0x62);
                bytes.extend_from_slice(&role.to_be_bytes());
            }
        }
    }

    pub fn encode_program(program: &[Instructions]) -> Vec<u8> {
        let mut bytes: Vec<u8> = vec![];
        for command in program {
            command.encode(&mut bytes)
        }
        bytes
    }
    pub fn decode(bytes: &[u8]) -> Result<(Instructions, usize), VMErrors> {
        let opcode = *bytes.first().ok_or(VMErrors::TruncatedOperand)?;
        match opcode {
            0x00 => Ok((Instructions::Halt, 1)),
            0x01 => Ok((Instructions::Revert, 1)),
            0x10 => Ok((Instructions::Push(read_u64(bytes)?), 9)),
            0x11 => Ok((Instructions::Pop, 1)),
            0x12 => Ok((Instructions::Double, 1)),
            0x13 => Ok((Instructions::Swap, 1)),
            0x20 => Ok((Instructions::Add, 1)),
            0x21 => Ok((Instructions::Subtraction, 1)),
            0x22 => Ok((Instructions::Product, 1)),
            0x23 => Ok((Instructions::Divide, 1)),
            0x30 => Ok((Instructions::Equal, 1)),
            0x31 => Ok((Instructions::Lower, 1)),
            0x32 => Ok((Instructions::Greater, 1)),
            0x40 => Ok((Instructions::Load(read_u8(bytes)?), 2)),
            0x41 => Ok((Instructions::Store(read_u8(bytes)?), 2)),
            0x50 => Ok((Instructions::Jump(read_u32(bytes)?), 5)),
            0x51 => Ok((Instructions::JumpIfZero(read_u32(bytes)?), 5)),
            0x60 => Ok((Instructions::PushAmount, 1)),
            0x61 => Ok((Instructions::LoadBalance(read_u8(bytes)?), 2)),
            0x62 => Ok((Instructions::StoreBalance(read_u8(bytes)?), 2)),
            other => Err(VMErrors::UnknownOpcode(other)),
        }
    }
    pub fn decode_program(bytes: &[u8]) -> Result<Vec<Instructions>, VMErrors> {
        if bytes.len() < 4097 {
            return Err(VMErrors::ProgramTooLong);
        }
        let mut res: Vec<Instructions> = vec![];
        let mut count: usize = 0;
        while count < bytes.len() {
            let reste = bytes.get(count..).ok_or(VMErrors::TruncatedOperand)?;
            let (instruction, size) = Instructions::decode(reste)?;
            res.push(instruction);
            count += size;
        }
        for instru in &res {
            match instru {
                Instructions::Jump(t)|Instructions::JumpIfZero(t) => {
                    if *t as usize >= res.len() {
                        return Err(VMErrors::InvalidJumpTarget(*t));
                    }}
                Instructions::Load(r)|Instructions::LoadBalance(r) =>{if *r>1 {
                       return  Err(InvalidRole(*r));
                    
                }
                _=>{}}
                
            
        }
        Ok(res)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn pop_on_empty_stack() {
        let mut vm = Vm::new(vec![]);
        assert_eq!(vm.pop(), Err(VMErrors::StackUnderflow));
    }

    #[test]
    fn push_257_values_overflows() {
        let mut vm = Vm::new(vec![]);
        for i in 0..256 {
            assert_eq!(vm.push(i), Ok(()));
        }
        assert_eq!(vm.push(256), Err(VMErrors::StackOverflow));
    }
}
