use std::collections::Btreemap;;
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum VMErrors{
    ProgramTooLong,
    UnknowOpcode(Vec<u8>),
    TruncatedOperand,
    InvalidRole(Vec<u8>),
    InvalidJumpTarget(Vec<u8>),
    StackUndeflow,
    StackOverflow,
    ArithmeticOverflow,
    DividedByZero,
    MemoryOutOfBounds(i),
    OutOfGas,
    Reverted,

}
pub enum Instructions{
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
pub struct Vm{
    Pile:Vec<u64>,
    Memory:[u64,16],
    Gas:u64,
    Context:Vec<(String,String,u64)>,
    PersistentState: BTreeMap<String,u64>,
}
impl Vm{
    pub fn new(Context:Vec<(String,String,u64)>)->Vm{
        let Pile = vec![];
        let Memory=[0;16];
        let Gas=10000;
        let Context=vec![];
        let PersistentState=BTreeMap::new();
        {
            Pile,
            Memory,
            Gas,
            Context,
            PersistentState,
        }

    }
    fn pop(&mut self)->Result<(),VMErrors>{
        self.Pile.pop().ok_or(VMError::StackUnderflow)
    }
    fn push(&mut self,elmt:u64 )->Result<(),VMErrors>{
        if self.Pile.len()>=256{return VMErrors::StackOverflow}
        else{self.Pile.push(u64)}
    }
}