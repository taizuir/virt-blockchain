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
    context: (String, String, u64),
    persistent_state: BTreeMap<String, u64>,
    journal:BTreeMap<String,u64>
    counter:usize,
}
impl Vm {
    pub fn new(context:(String, String, u64),gas_given:u64) -> Vm {
        let pile = ();
        let memory = [0; 16];
        let gas = gas_given;
        let persistent_state = BTreeMap::new();
        let counter=0;
        Vm {
            pile,
            memory,
            gas,
            context,
            persistent_state,
        }
    }
    fn gas_comsumption(instruction: &Instructions) -> u64 {
        match instruction {
            Instructions::Halt | Instructions::Revert => 0,
            Instructions::Push(_)
            | Instructions::Pop
            | Instructions::Double
            | Instructions::Swap
            | Instructions::PushAmount => 1,
            Instructions::Add
            | Instructions::Subtraction
            | Instructions::Equal
            | Instructions::Lower
            | Instructions::Greater
            | Instructions::Jump(_)
            | Instructions::JumpIfZero(_) => 2,
            Instructions::Product
            | Instructions::Divide
            | Instructions::Load(_)
            | Instructions::Store(_) => 3,
            Instructions::LoadBalance(_) => 5,
            Instructions::StoreBalance(_) => 20,
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
    fn run(&mut self, program: &[Instructions]) -> Result<(), VMErrors> {
        let mut counter: usize = 0;
        while let Some(instruction) = program.get(counter) {
            self.gas = self.gas.checked_sub(Vm::gas_comsumption(instruction)).ok_or(VMErrors::OutOfGas)?
            let mut next = counter + 1;
            match instruction {
                Instructions::Push(value) => {
                    self.push(*value)?;
                }
                Instructions::Pop => {
                    self.pop()?;
                }
                Instructions::Double => {
                    let x = *self.pile.last().ok_or(VMErrors::StackUnderflow)?;
                    self.push(x)?;
                }
                Instructions::Add => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(a.checked_add(b).ok_or(VMErrors::ArithmeticOverflow)?)?;
                }
                Instructions::Equal =>  {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(if a == b { 1 } else { 0 })?;
                },
                Instructions::Lower => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(if a < b { 1 } else { 0 })?;
                },
                Instructions::Greater => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(if a > b { 1 } else { 0 })?;
                },
                Instrucion::Swap=>{let b= self.pop()?; let a = self.pop()?;self.push(b)?;self.push(a)?}
                Instruciton::Add=>{let b= self.pop()?; let a = self.pop()?;self.push(a.checked_add(b).ok_or(VMErrors::ArithmeticOverflow))}
                Instruction::Substraction=>{let b= self.pop()?; let a = self.pop();self.push(a.checked_sub().ok_or(VMErrors::ArithmeticOverflow))}
                Instrucion::Divide=>{let b = self.pop()?;let a = self.pop()?;self.push(a.checked_div(b).ok_or(VMErrors::DivisionByZero)?)?},
                Instruction::Product=>{let b= self.pop()?; let a = self.pop();self.push(a.checked_mul(b).ok_or(VMErrors::ArithmeticOverflow))}
                Instructions::Load(cell) => {
                    let index = *cell as usize;
                    if index >= self.memory.len() {
                        return Err(VMErrors::MemoryOutOfBounds(index));
                    }
                    let value = self.memory[index];
                    self.push(value)?;
                },
                Instructions::Store(cell) => {
                    let index = *cell as usize;
                    if index >= self.memory.len() {
                        return Err(VMErrors::MemoryOutOfBounds(index));
                    }
                    let x = self.pop()?;
                    self.memory[index] = x;
                },
                Instructions::Jump(target) => next = *target as usize,
                Instructions::JumpIfZero(target) => {
                    let x = self.pop()?;
                    if x == 0 {
                        next = *target as usize;
                    }
                },
                Instructions::Halt => return Ok(()),
                Instructions::Revert => return Err(VMErrors::Reverted),
                Instructions::PushAmount => {self.push(self.context.2)?},
                Instructions::LoadBalance(role) =>{
                    let address = self.account(*role)?;
                    let balance = self
                        .journal
                        .get(address)
                        .or(self.persistent_state.get(address))
                        .copied()
                        .unwrap_or(0);
                    self.push(balance)?;
                },
                Instructions::StoreBalance(role) =>{
                    let x = self.pop()?;
                    let address = self.account(*role)?.clone();
                    self.journal.insert(address, x);
                }
                gas
            }
            counter = next;
        }
        Ok(())
    }
    }
    fn execute(&mut self){}
    
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
