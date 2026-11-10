use std::collections::BTreeMap;

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
    journal:BTreeMap<String,u64>,
    counter:usize,
}
impl Vm {
    pub fn new(context:(String, String, u64),gas_given:u64) -> Vm {
        let pile = Vec::new();
        let memory = [0; 16];
        let gas = gas_given;
        let persistent_state = BTreeMap::new();

        let journal = BTreeMap::new();
        let counter=0;
        Vm {
            pile,
            memory,
            gas,
            context,
            persistent_state,
            journal,
            counter,
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
    // rôle 0 = expéditeur, rôle 1 = destinataire
    fn account(&self, role: u8) -> Result<&String, VMErrors> {
        match role {
            0 => Ok(&self.context.0),
            1 => Ok(&self.context.1),
            _ => Err(VMErrors::InvalidRole(role)),
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
            self.gas = self.gas.checked_sub(Vm::gas_comsumption(instruction)).ok_or(VMErrors::OutOfGas)?;
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
                Instructions::Swap => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(b)?;
                    self.push(a)?;
                }
                Instructions::Subtraction => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(a.checked_sub(b).ok_or(VMErrors::ArithmeticOverflow)?)?;
                }
                Instructions::Divide => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(a.checked_div(b).ok_or(VMErrors::DivisionByZero)?)?;
                }
                Instructions::Product => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.push(a.checked_mul(b).ok_or(VMErrors::ArithmeticOverflow)?)?;
                }
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
            }
            counter = next;
        }
        Ok(())
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
        if bytes.len()> 4096{
            return Err(VMErrors::ProgramTooLong);
        }
        let mut res: Vec<Instructions> = vec![];
        let mut count = 0;

        while count < bytes.len() {
            let reste = bytes.get(count..).ok_or(VMErrors::TruncatedOperand)?;
            let (instruction, size) = Instructions::decode(reste)?;
            res.push(instruction);
            count += size;
        }
        for instruction in &res {
            match instruction {
                Instructions::JumpIfZero(t) | Instructions::Jump(t) => {
                    if *t as usize >= res.len() {
                        return Err(VMErrors::InvalidJumpTarget(*t));
                    }
                }
                Instructions::LoadBalance(r) | Instructions::StoreBalance(r) => {
                    if *r > 1 {
                        return Err(VMErrors::InvalidRole(*r));
                    }
                }
                _ => {}
            }
        }
        Ok(res)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use Instructions::*;

    fn test_vm() -> Vm {
        Vm::new(("alice".to_string(), "bob".to_string(), 30),1000)
    }

    fn pile(program: &[Instructions]) -> Vec<u64> {
        let mut vm = test_vm();
        vm.run(program).unwrap();
        vm.pile
    }

    fn error(program: &[Instructions]) -> VMErrors {
        test_vm().run(program).unwrap_err()
    }

    fn vm_with_state(alice: u64, bob: u64) -> Vm {
        let mut vm = test_vm();
        vm.persistent_state.insert("alice".to_string(), alice);
        vm.persistent_state.insert("bob".to_string(), bob);
        vm
    }

    #[test]
    fn pop_on_empty_stack() {
        let mut vm = test_vm();
        assert_eq!(vm.pop(), Err(VMErrors::StackUnderflow));
    }

    #[test]
    fn push_257_values_overflows() {
        let mut vm = test_vm();
        for i in 0..256 {
            assert_eq!(vm.push(i), Ok(()));
        }
        assert_eq!(vm.push(256), Err(VMErrors::StackOverflow));
    }
    fn all_instructions() -> Vec<Instructions> {
        vec![
            Instructions::Halt,
            Instructions::Revert,
            Instructions::Push(42),
            Instructions::Pop,
            Instructions::Double,
            Instructions::Swap,
            Instructions::Add,
            Instructions::Subtraction,
            Instructions::Product,
            Instructions::Divide,
            Instructions::Equal,
            Instructions::Lower,
            Instructions::Greater,
            Instructions::Load(3),
            Instructions::Store(4),
            Instructions::Jump(0),
            Instructions::JumpIfZero(1),
            Instructions::PushAmount,
            Instructions::LoadBalance(0),
            Instructions::StoreBalance(1),
        ]
    }

    fn encoded(instruction: Instructions) -> Vec<u8> {
        let mut bytes = vec![];
        instruction.encode(&mut bytes);
        bytes
    }

    #[test]
    fn first_byte_of_each_instruction() {
        let table = [
            (Instructions::Halt, 0x00),
            (Instructions::Revert, 0x01),
            (Instructions::Push(0), 0x10),
            (Instructions::Pop, 0x11),
            (Instructions::Double, 0x12),
            (Instructions::Swap, 0x13),
            (Instructions::Add, 0x20),
            (Instructions::Subtraction, 0x21),
            (Instructions::Product, 0x22),
            (Instructions::Divide, 0x23),
            (Instructions::Equal, 0x30),
            (Instructions::Lower, 0x31),
            (Instructions::Greater, 0x32),
            (Instructions::Load(0), 0x40),
            (Instructions::Store(0), 0x41),
            (Instructions::Jump(0), 0x50),
            (Instructions::JumpIfZero(0), 0x51),
            (Instructions::PushAmount, 0x60),
            (Instructions::LoadBalance(0), 0x61),
            (Instructions::StoreBalance(0), 0x62),
        ];
        for (instruction, opcode) in table {
            assert_eq!(encoded(instruction)[0], opcode, "{:?}", instruction);
        }
    }

    #[test]
    fn opcodes_are_all_different() {
        let mut seen = vec![];
        for instruction in all_instructions() {
            let opcode = encoded(instruction)[0];
            assert!(!seen.contains(&opcode), "doublon {:#04x}", opcode);
            seen.push(opcode);
        }
        assert_eq!(seen.len(), 20);
    }

    #[test]
    fn encoded_sizes() {
        assert_eq!(encoded(Instructions::Add).len(), 1);
        assert_eq!(encoded(Instructions::Load(3)).len(), 2);
        assert_eq!(encoded(Instructions::LoadBalance(1)).len(), 2);
        assert_eq!(encoded(Instructions::Jump(5)).len(), 5);
        assert_eq!(encoded(Instructions::Push(7)).len(), 9);
    }

    #[test]
    fn encode_push_one() {
        assert_eq!(
            encoded(Instructions::Push(1)),
            vec![0x10, 0, 0, 0, 0, 0, 0, 0, 1]
        );
    }

    #[test]
    fn encode_push_is_big_endian() {
        assert_eq!(
            encoded(Instructions::Push(0x0102030405060708)),
            vec![0x10, 1, 2, 3, 4, 5, 6, 7, 8]
        );
    }

    #[test]
    fn encode_push_max() {
        let mut expected = vec![0x10];
        expected.extend_from_slice(&[0xFF; 8]);
        assert_eq!(encoded(Instructions::Push(u64::MAX)), expected);
    }

    #[test]
    fn encode_jump_five() {
        assert_eq!(encoded(Instructions::Jump(5)), vec![0x50, 0, 0, 0, 5]);
    }

    #[test]
    fn encode_jump_is_big_endian() {
        assert_eq!(
            encoded(Instructions::JumpIfZero(0x01020304)),
            vec![0x51, 1, 2, 3, 4]
        );
    }

    #[test]
    fn encode_load_three() {
        assert_eq!(encoded(Instructions::Load(3)), vec![0x40, 3]);
    }

    #[test]
    fn encode_empty_program() {
        assert_eq!(Instructions::encode_program(&[]), Vec::<u8>::new());
    }

    #[test]
    fn encode_sample_program() {
        let program = [
            Instructions::Push(7),
            Instructions::Push(3),
            Instructions::Add,
            Instructions::Halt,
        ];
        let bytes = Instructions::encode_program(&program);
        assert_eq!(bytes.len(), 20);
        assert_eq!(bytes[0], 0x10);
        assert_eq!(bytes[8], 7);
        assert_eq!(bytes[9], 0x10);
        assert_eq!(bytes[17], 3);
        assert_eq!(bytes[18], 0x20);
        assert_eq!(bytes[19], 0x00);
    }

    #[test]
    fn decode_one_instruction_of_each_size() {
        assert_eq!(Instructions::decode(&[0x00]), Ok((Instructions::Halt, 1)));
        assert_eq!(
            Instructions::decode(&[0x40, 3]),
            Ok((Instructions::Load(3), 2))
        );
        assert_eq!(
            Instructions::decode(&[0x50, 0, 0, 0, 5]),
            Ok((Instructions::Jump(5), 5))
        );
        assert_eq!(
            Instructions::decode(&[0x10, 0, 0, 0, 0, 0, 0, 0, 7]),
            Ok((Instructions::Push(7), 9))
        );
    }

    #[test]
    fn decode_ignores_what_follows() {
        assert_eq!(
            Instructions::decode(&[0x20, 0x20, 0x20]),
            Ok((Instructions::Add, 1))
        );
    }

    #[test]
    fn decode_empty_slice_is_an_error() {
        assert_eq!(Instructions::decode(&[]), Err(VMErrors::TruncatedOperand));
    }

    #[test]
    fn unknown_opcode() {
        assert_eq!(
            Instructions::decode_program(&[0xFF]),
            Err(VMErrors::UnknownOpcode(255))
        );
        assert_eq!(
            Instructions::decode_program(&[0x14]),
            Err(VMErrors::UnknownOpcode(0x14))
        );
    }

    #[test]
    fn truncated_operand() {
        assert_eq!(
            Instructions::decode_program(&[0x10, 1, 2, 3]),
            Err(VMErrors::TruncatedOperand)
        );
        assert_eq!(
            Instructions::decode_program(&[0x50, 0, 0]),
            Err(VMErrors::TruncatedOperand)
        );
        assert_eq!(
            Instructions::decode_program(&[0x40]),
            Err(VMErrors::TruncatedOperand)
        );
    }

    #[test]
    fn truncated_operand_after_valid_instruction() {
        assert_eq!(
            Instructions::decode_program(&[0x20, 0x10, 1, 2]),
            Err(VMErrors::TruncatedOperand)
        );
    }

    #[test]
    fn invalid_role() {
        assert_eq!(
            Instructions::decode_program(&[0x61, 0x02]),
            Err(VMErrors::InvalidRole(2))
        );
        assert_eq!(
            Instructions::decode_program(&[0x62, 0xFF]),
            Err(VMErrors::InvalidRole(255))
        );
    }

    #[test]
    fn valid_roles() {
        assert!(Instructions::decode_program(&[0x61, 0x00, 0x62, 0x01]).is_ok());
    }

    #[test]
    fn invalid_jump_target() {
        let bytes = Instructions::encode_program(&[Instructions::Jump(5), Instructions::Halt]);
        assert_eq!(
            Instructions::decode_program(&bytes),
            Err(VMErrors::InvalidJumpTarget(5))
        );
    }

    #[test]
    fn jump_target_boundary() {
        let last = Instructions::encode_program(&[Instructions::Jump(1), Instructions::Halt]);
        assert!(Instructions::decode_program(&last).is_ok());
        let past = Instructions::encode_program(&[Instructions::Jump(2), Instructions::Halt]);
        assert_eq!(
            Instructions::decode_program(&past),
            Err(VMErrors::InvalidJumpTarget(2))
        );
    }

    #[test]
    fn jump_if_zero_target_is_checked_too() {
        let bytes = Instructions::encode_program(&[Instructions::JumpIfZero(9)]);
        assert_eq!(
            Instructions::decode_program(&bytes),
            Err(VMErrors::InvalidJumpTarget(9))
        );
    }

    #[test]
    fn jump_target_counts_instructions_not_bytes() {
        let program = [
            Instructions::Push(1),
            Instructions::Push(2),
            Instructions::Jump(2),
        ];
        let bytes = Instructions::encode_program(&program);
        assert_eq!(Instructions::decode_program(&bytes), Ok(program.to_vec()));
    }

    #[test]
    fn program_of_4096_bytes_is_accepted() {
        let bytes = vec![0x00; 4096];
        let program = Instructions::decode_program(&bytes).unwrap();
        assert_eq!(program.len(), 4096);
    }

    #[test]
    fn program_of_4097_bytes_is_too_long() {
        let bytes = vec![0x00; 4097];
        assert_eq!(
            Instructions::decode_program(&bytes),
            Err(VMErrors::ProgramTooLong)
        );
    }

    #[test]
    fn too_long_is_checked_before_anything_else() {
        let bytes = vec![0xFF; 5000];
        assert_eq!(
            Instructions::decode_program(&bytes),
            Err(VMErrors::ProgramTooLong)
        );
    }

    #[test]
    fn empty_program_is_valid() {
        assert_eq!(Instructions::decode_program(&[]), Ok(vec![]));
    }

    #[test]
    fn round_trip_all_instructions() {
        let program = all_instructions();
        let bytes = Instructions::encode_program(&program);
        assert_eq!(Instructions::decode_program(&bytes), Ok(program));
    }

    #[test]
    fn round_trip_extreme_operands() {
        let program = vec![
            Instructions::Push(0),
            Instructions::Push(u64::MAX),
            Instructions::Load(15),
            Instructions::Store(255),
            Instructions::Jump(0),
            Instructions::JumpIfZero(4),
            Instructions::LoadBalance(1),
            Instructions::StoreBalance(0),
        ];
        let bytes = Instructions::encode_program(&program);
        assert_eq!(Instructions::decode_program(&bytes), Ok(program));
    }

    #[test]
    fn decoding_never_panics_on_any_prefix() {
        let bytes = Instructions::encode_program(&all_instructions());
        for end in 0..=bytes.len() {
            let _ = Instructions::decode_program(&bytes[..end]);
        }
    }

    #[test]
    fn decoding_never_panics_on_pseudo_random_bytes() {
        let mut state: u64 = 0x2545F4914F6CDD1D;
        for _ in 0..2000 {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            let len = (state % 40) as usize;
            let mut bytes = vec![];
            for k in 0..len {
                bytes.push((state >> (k % 8 * 8)) as u8 ^ k as u8);
            }
            let _ = Instructions::decode_program(&bytes);
        }
    }

    #[test]
    fn stack_instructions() {
        assert_eq!(pile(&[]), vec![]);
        assert_eq!(pile(&[Push(7)]), vec![7]);
        assert_eq!(pile(&[Push(1), Push(2), Push(3)]), vec![1, 2, 3]);
        assert_eq!(pile(&[Push(1), Push(2), Pop]), vec![1]);
        assert_eq!(pile(&[Push(1), Push(5), Double]), vec![1, 5, 5]);
        assert_eq!(pile(&[Push(1), Push(2), Swap]), vec![2, 1]);
        assert_eq!(
            pile(&[Push(1), Push(2), Push(3), Push(4), Swap]),
            vec![1, 2, 4, 3]
        );
        assert_eq!(pile(&[Push(1), Push(2), Swap, Swap]), vec![1, 2]);
        assert_eq!(pile(&[Push(1), Push(2), Swap, Double]), vec![2, 1, 1]);
    }

    #[test]
    fn stack_errors() {
        assert_eq!(error(&[Pop]), VMErrors::StackUnderflow);
        assert_eq!(error(&[Double]), VMErrors::StackUnderflow);
        assert_eq!(error(&[Swap]), VMErrors::StackUnderflow);
        assert_eq!(error(&[Push(1), Swap]), VMErrors::StackUnderflow);
        assert_eq!(pile(&vec![Push(1); 256]).len(), 256);
        assert_eq!(error(&vec![Push(1); 257]), VMErrors::StackOverflow);
        let mut full = vec![Push(1); 256];
        full.push(Double);
        assert_eq!(error(&full), VMErrors::StackOverflow);
    }

    #[test]
    fn arithmetic_results() {
        assert_eq!(pile(&[Push(7), Push(3), Add]), vec![10]);
        assert_eq!(pile(&[Push(10), Push(3), Subtraction]), vec![7]);
        assert_eq!(pile(&[Push(3), Push(3), Subtraction]), vec![0]);
        assert_eq!(pile(&[Push(6), Push(7), Product]), vec![42]);
        assert_eq!(pile(&[Push(100), Push(10), Divide]), vec![10]);
        assert_eq!(pile(&[Push(7), Push(2), Divide]), vec![3]);
        assert_eq!(pile(&[Push(0), Push(5), Divide]), vec![0]);
        assert_eq!(pile(&[Push(u64::MAX), Push(0), Add]), vec![u64::MAX]);
        assert_eq!(pile(&[Push(u64::MAX), Push(0), Product]), vec![0]);
    }

    #[test]
    fn arithmetic_errors() {
        assert_eq!(
            error(&[Push(u64::MAX), Push(1), Add]),
            VMErrors::ArithmeticOverflow
        );
        assert_eq!(
            error(&[Push(2), Push(3), Subtraction]),
            VMErrors::ArithmeticOverflow
        );
        assert_eq!(
            error(&[Push(u64::MAX), Push(2), Product]),
            VMErrors::ArithmeticOverflow
        );
        assert_eq!(error(&[Push(5), Push(0), Divide]), VMErrors::DivisionByZero);
    }

    #[test]
    fn binary_operations_need_two_values() {
        for operation in [Add, Subtraction, Product, Divide, Equal, Lower, Greater] {
            assert_eq!(error(&[operation]), VMErrors::StackUnderflow);
            assert_eq!(error(&[Push(1), operation]), VMErrors::StackUnderflow);
        }
    }

    #[test]
    fn comparisons() {
        assert_eq!(pile(&[Push(1), Push(2), Lower]), vec![1]);
        assert_eq!(pile(&[Push(2), Push(1), Lower]), vec![0]);
        assert_eq!(pile(&[Push(2), Push(2), Lower]), vec![0]);
        assert_eq!(pile(&[Push(5), Push(4), Greater]), vec![1]);
        assert_eq!(pile(&[Push(4), Push(5), Greater]), vec![0]);
        assert_eq!(pile(&[Push(4), Push(4), Greater]), vec![0]);
        assert_eq!(pile(&[Push(4), Push(4), Equal]), vec![1]);
        assert_eq!(pile(&[Push(4), Push(5), Equal]), vec![0]);
    }

    #[test]
    fn memory() {
        assert_eq!(pile(&[Push(5), Store(0), Load(0)]), vec![5]);
        assert_eq!(pile(&[Push(5), Store(3), Load(3), Load(3)]), vec![5, 5]);
        assert_eq!(pile(&[Load(0), Load(15)]), vec![0, 0]);
        assert_eq!(pile(&[Push(9), Store(15), Load(15)]), vec![9]);
        assert_eq!(
            pile(&[Push(1), Store(0), Push(2), Store(0), Load(0)]),
            vec![2]
        );
    }

    #[test]
    fn memory_errors() {
        assert_eq!(error(&[Load(16)]), VMErrors::MemoryOutOfBounds(16));
        assert_eq!(error(&[Load(255)]), VMErrors::MemoryOutOfBounds(255));
        assert_eq!(
            error(&[Push(1), Store(16)]),
            VMErrors::MemoryOutOfBounds(16)
        );
        assert_eq!(error(&[Store(0)]), VMErrors::StackUnderflow);
        let mut full = vec![Push(1); 256];
        full.push(Load(0));
        assert_eq!(error(&full), VMErrors::StackOverflow);
    }

    #[test]
    fn jumps() {
        assert_eq!(pile(&[Jump(2), Push(1), Push(2)]), vec![2]);
        assert_eq!(pile(&[Push(1), Jump(3), Push(2)]), vec![1]);
        assert_eq!(pile(&[Push(0), JumpIfZero(3), Push(7), Halt]), vec![]);
        assert_eq!(pile(&[Push(1), JumpIfZero(3), Push(7), Halt]), vec![7]);
        assert_eq!(pile(&[Push(0), JumpIfZero(2), Push(7), Halt]), vec![7]);
        assert_eq!(pile(&[Push(5), Push(1), JumpIfZero(4)]), vec![5]);
        assert_eq!(error(&[JumpIfZero(0)]), VMErrors::StackUnderflow);
    }

    #[test]
    fn halt_revert_and_end_of_program() {
        assert_eq!(pile(&[Push(1), Halt, Push(2)]), vec![1]);
        assert_eq!(pile(&[Push(1), Push(2)]), vec![1, 2]);
        assert_eq!(error(&[Revert]), VMErrors::Reverted);
        assert_eq!(error(&[Push(1), Revert, Push(2)]), VMErrors::Reverted);
    }

    #[test]
    fn long_program_with_jumps_and_memory() {
        let mut program = [
            Push(10),
            Push(3),
            Swap,
            Store(0),
            Push(4),
            Add,
            Load(0),
            Lower,
            JumpIfZero(11),
            Push(99),
            Halt,
            Push(0),
            Halt,
        ];
        assert_eq!(pile(&program), vec![99]);
        program[0] = Push(5);
        assert_eq!(pile(&program), vec![0]);
    }

    #[test]
    fn countdown_loop() {
        let program = [
            Push(3),
            Store(0),
            Load(0),
            JumpIfZero(10),
            Load(0),
            Push(1),
            Subtraction,
            Store(0),
            Jump(2),
            Halt,
            Push(42),
        ];
        assert_eq!(pile(&program), vec![42]);
    }

    #[test]
    fn context_and_roles() {
        assert_eq!(pile(&[PushAmount]), vec![30]);
        let mut full = vec![Push(1); 256];
        full.push(PushAmount);
        assert_eq!(error(&full), VMErrors::StackOverflow);
        let vm = test_vm();
        assert_eq!(vm.account(0), Ok(&"alice".to_string()));
        assert_eq!(vm.account(1), Ok(&"bob".to_string()));
        assert_eq!(vm.account(2), Err(VMErrors::InvalidRole(2)));
        assert_eq!(error(&[LoadBalance(2)]), VMErrors::InvalidRole(2));
        assert_eq!(error(&[Push(1), StoreBalance(2)]), VMErrors::InvalidRole(2));
        assert_eq!(error(&[StoreBalance(0)]), VMErrors::StackUnderflow);
    }

    #[test]
    fn balances_and_journal() {
        assert_eq!(pile(&[LoadBalance(0), LoadBalance(1)]), vec![0, 0]);
        let mut vm = vm_with_state(100, 20);
        vm.run(&[LoadBalance(0), LoadBalance(1)]).unwrap();
        assert_eq!(vm.pile, vec![100, 20]);
        let mut vm = vm_with_state(100, 20);
        vm.run(&[Push(70), StoreBalance(0)]).unwrap();
        assert_eq!(vm.journal.get("alice"), Some(&70));
        assert_eq!(vm.persistent_state.get("alice"), Some(&100));
        let mut vm = vm_with_state(100, 20);
        vm.run(&[Push(70), StoreBalance(0), LoadBalance(0)])
            .unwrap();
        assert_eq!(vm.pile, vec![70]);
        let mut vm = test_vm();
        vm.run(&[Push(1), StoreBalance(1), Push(2), StoreBalance(1)])
            .unwrap();
        assert_eq!(vm.journal.get("bob"), Some(&2));
        assert_eq!(vm.journal.len(), 1);
    }

    #[test]
    fn transfer_programs() {
        let transfer = [
            LoadBalance(0),
            PushAmount,
            Subtraction,
            StoreBalance(0),
            LoadBalance(1),
            PushAmount,
            Add,
            StoreBalance(1),
            Halt,
        ];
        let mut vm = vm_with_state(100, 20);
        vm.run(&transfer).unwrap();
        assert_eq!(vm.journal.get("alice"), Some(&70));
        assert_eq!(vm.journal.get("bob"), Some(&50));
        let mut poor = vm_with_state(10, 20);
        assert_eq!(poor.run(&transfer), Err(VMErrors::ArithmeticOverflow));
        assert!(poor.journal.is_empty());
    }

    #[test]
    fn conditional_transfer_reverts_when_funds_are_missing() {
        let program = [
            LoadBalance(0),
            PushAmount,
            Lower,
            JumpIfZero(5),
            Revert,
            Halt,
        ];
        assert_eq!(vm_with_state(10, 0).run(&program), Err(VMErrors::Reverted));
        assert_eq!(vm_with_state(100, 0).run(&program), Ok(()));
    }
}
