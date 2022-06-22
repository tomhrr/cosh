/// The opcodes used in the compiler and the bytecode.
#[derive(Debug, Clone, Copy)]
pub enum OpCode {
    Constant = 1,
    Add = 2,
    Subtract = 3,
    Multiply = 4,
    Divide = 5,
    EndFn = 6,
    Call = 7,
    CallImplicit = 8,
    Function = 9,
    Var = 10,
    SetVar = 11,
    GetVar = 12,
    SetLocalVar = 13,
    GetLocalVar = 14,
    PopLocalVar = 15,
    Jump = 16,
    JumpNe = 17,
    JumpNeR = 18,
    Eq = 19,
    Gt = 20,
    Lt = 21,
    Print = 22,
    Dup = 23,
    Swap = 24,
    Drop = 25,
    Rot = 26,
    Over = 27,
    Depth = 28,
    Clear = 29,
    StartList = 30,
    EndList = 31,
    StartHash = 32,
    Shift = 33,
    Yield = 34,
    IsNull = 35,
    IsList = 36,
    IsCallable = 37,
    IsShiftable = 38,
    Open = 39,
    Readline = 40,
    Error = 41,
    Return = 42,
    Str = 43,
    Int = 44,
    Flt = 45,
    Rand = 46,
    AddConstant = 47,
    EqConstant = 48,
    JumpNeREqC = 49,
    Push = 50,
    Pop = 51,
    DupIsNull = 52,
    Unknown = 255,
}

/// Convert a byte to an opcode value.
pub fn to_opcode(value: u8) -> OpCode {
    match value {
        1 => OpCode::Constant,
        2 => OpCode::Add,
        3 => OpCode::Subtract,
        4 => OpCode::Multiply,
        5 => OpCode::Divide,
        6 => OpCode::EndFn,
        7 => OpCode::Call,
        8 => OpCode::CallImplicit,
        9 => OpCode::Function,
        10 => OpCode::Var,
        11 => OpCode::SetVar,
        12 => OpCode::GetVar,
        13 => OpCode::SetLocalVar,
        14 => OpCode::GetLocalVar,
        15 => OpCode::PopLocalVar,
        16 => OpCode::Jump,
        17 => OpCode::JumpNe,
        18 => OpCode::JumpNeR,
        19 => OpCode::Eq,
        20 => OpCode::Gt,
        21 => OpCode::Lt,
        22 => OpCode::Print,
        23 => OpCode::Dup,
        24 => OpCode::Swap,
        25 => OpCode::Drop,
        26 => OpCode::Rot,
        27 => OpCode::Over,
        28 => OpCode::Depth,
        29 => OpCode::Clear,
        30 => OpCode::StartList,
        31 => OpCode::EndList,
        32 => OpCode::StartHash,
        33 => OpCode::Shift,
        34 => OpCode::Yield,
        35 => OpCode::IsNull,
        36 => OpCode::IsList,
        37 => OpCode::IsCallable,
        38 => OpCode::IsShiftable,
        39 => OpCode::Open,
        40 => OpCode::Readline,
        41 => OpCode::Error,
        42 => OpCode::Return,
        43 => OpCode::Str,
        44 => OpCode::Int,
        45 => OpCode::Flt,
        46 => OpCode::Rand,
        47 => OpCode::AddConstant,
        48 => OpCode::EqConstant,
        49 => OpCode::JumpNeREqC,
        50 => OpCode::Push,
        51 => OpCode::Pop,
        52 => OpCode::DupIsNull,
        255 => OpCode::Unknown,
        _ => OpCode::Unknown,
    }
}
