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
