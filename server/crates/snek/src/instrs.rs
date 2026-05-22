#[derive(Debug)]
pub(crate) enum Reg {
    RAX,
    RBX,
    RCX,
    RDX,
    RSP,
    RBP,
    RDI,
    R10,
    R11,
    R15,
    RSI,
}

pub const RAX: Reg = Reg::RAX;
pub const RBX: Reg = Reg::RBX;
pub const RCX: Reg = Reg::RCX;
pub const RDX: Reg = Reg::RDX;
pub const RSP: Reg = Reg::RSP;
pub const RBP: Reg = Reg::RBP;
pub const RDI: Reg = Reg::RDI;
pub const RSI: Reg = Reg::RSI;
pub const R10: Reg = Reg::R10;
pub const R11: Reg = Reg::R11;
pub const R15: Reg = Reg::R15;

impl Reg {
    /// Converts this register into a register-valued assembly operand.
    pub fn to_value(self) -> Val {
        Val::Reg(self)
    }

    /// Returns the x86-64 assembly name for this register.
    pub fn to_string(&self) -> String {
        match &self {
            Reg::RAX => format!("rax"),
            Reg::RBX => format!("rbx"),
            Reg::RCX => format!("rcx"),
            Reg::RDX => format!("rdx"),
            Reg::RSP => format!("rsp"),
            Reg::RBP => format!("rbp"),
            Reg::RDI => format!("rdi"),
            Reg::RSI => format!("rsi"),
            Reg::R10 => format!("r10"),
            Reg::R11 => format!("r11"),
            Reg::R15 => format!("r15"),
        }
    }
}

impl From<Reg> for Val {
    fn from(r: Reg) -> Self {
        r.to_value()
    }
}

#[derive(Debug)]
pub(crate) enum COp {
    EQ,
    NE,
    L,
    LE,
    G,
    GE,
    Z,
    NZ,
    O,
}

pub const EQ: COp = COp::EQ;
pub const NE: COp = COp::NE;
pub const LE: COp = COp::LE;
pub const GE: COp = COp::GE;
pub const L: COp = COp::L;
pub const G: COp = COp::G;
pub const Z: COp = COp::Z;
pub const NZ: COp = COp::NZ;
pub const O: COp = COp::O;

impl COp {
    /// Converts this condition code into a condition-code assembly operand.
    pub fn to_value(self) -> Val {
        Val::CompOp(self)
    }

    /// Returns the x86-64 condition-code suffix for this comparison.
    pub fn to_string(&self) -> String {
        match &self {
            COp::EQ => format!("e"),
            COp::NE => format!("ne"),
            COp::L => format!("l"),
            COp::LE => format!("le"),
            COp::G => format!("g"),
            COp::GE => format!("ge"),
            COp::Z => format!("z"),
            COp::NZ => format!("nz"),
            COp::O => format!("o"),
        }
    }
}

/// Val are the basic elements of asm code
///
#[derive(Debug)]
pub(crate) enum Val {
    Reg(Reg),
    RawNum(i64), // number without shift left
    Num(i64),    // tagged number, n << 1
    RegOffset(Reg, i64),
    PosOffset(Reg, i64),
    Hex(String),
    CompOp(COp),
    Label(String),
    Vector(Vec<Val>),
    // ErrCode
}

pub const TRUE: Val = Val::RawNum(7);
pub const FALSE: Val = Val::RawNum(3);
pub const NIL: Val = Val::RawNum(9);

impl Val {
    /// Formats this operand as x86-64 assembly text.
    pub fn to_string(&self) -> String {
        match &self {
            Val::Reg(reg) => reg.to_string(),
            Val::RawNum(n) => format!("{}", n),
            Val::Num(n) => format!("{}", n << 1),
            Val::RegOffset(reg, n) => format!("[{} - 8*{}]", reg.to_string(), n),
            Val::CompOp(cop) => cop.to_string(),
            Val::Label(lbl) => format!("{lbl}"),
            Val::Vector(_vals) => todo!(),
            Val::PosOffset(reg, n) => format!("[{} + {}]", reg.to_string(), n),
            Val::Hex(s) => s.to_string(),
        }
    }
}

/// Creates a stack-style memory operand at `[r - 8*n]`.
///
/// This is primarily used for compiler-managed stack slots relative to `rbp`.
///
/// # Examples
///
/// ```asm
/// [rbp - 8*1]
/// ```
pub fn offset(r: Reg, n: i64) -> Val {
    Val::RegOffset(r, n)
}

/// Creates a positive byte-offset memory operand at `[r + n]`.
///
/// The offset must be a multiple of 8 because the compiler stores values in
/// word-sized slots.
///
/// # Panics
///
/// Panics if `n` is not a multiple of 8.
///
/// # Examples
///
/// ```asm
/// [r + n]
/// ```
pub fn byte_offset(r: Reg, n: i64) -> Val {
    if (n % 8) != 0 {
        panic!("n must be multiple of 8")
    }
    Val::PosOffset(r, n)
}

/// Creates an immediate operand using the compiler's tagged number encoding.
///
/// The emitted assembly value is `n << 1`.
pub fn tagged(n: i64) -> Val {
    Val::Num(n)
}

/// Creates an immediate operand without applying any runtime tag encoding.
pub fn untagged(n: i64) -> Val {
    Val::RawNum(n)
}

#[derive(Debug)]
pub(crate) enum Instr {
    IMov(Val, Val),
    IAdd(Val, Val),
    ISub(Val, Val),
    IMul(Val, Val),
    ICmp(Val, Val),
    // Conditional move, use asm condition as string i.e., cmove, cmovne,...
    ICMov(Val, Val, Val),
    // je jne jg jge jl jle jo
    IJump(Val, Val),
    IAnd(Val, Val),
    ILabel(Val),
    ITest(Val, Val),
    IXor(Val, Val),
    ISar(Val, Val),
    IShl(Val, Val),
    ILea(Val, Val),
    IJmp(Val),
    ICall(Val),
    IPush(Val),
    IPop(Val),
    IRaw(String),
    IRet,
}

/// Lowering instruction `mov` in asm
/// Moves `v2` into `v1`.
///
/// ## Example
///
/// ```rust,ignore
/// imov(RAX.into(), 100)
/// ```
///
/// lowers the assembly code:
///
/// ```asm
/// mov rax, 100
/// ```
///
/// which moves 100 into rax.
pub fn imov(v1: Val, v2: Val) -> Instr {
    Instr::IMov(v1, v2)
}

/// Creates an `add` instruction that adds `v2` into `v1`.
pub fn iadd(v1: Val, v2: Val) -> Instr {
    Instr::IAdd(v1, v2)
}

/// Creates a `sub` instruction that subtracts `v2` from `v1`.
pub fn isub(v1: Val, v2: Val) -> Instr {
    Instr::ISub(v1, v2)
}

/// Creates an `imul` instruction that multiplies `v1` by `v2`.
pub fn imul(v1: Val, v2: Val) -> Instr {
    Instr::IMul(v1, v2)
}

/// Creates a `cmp` instruction comparing `v1` and `v2`.
pub fn cmp(v1: Val, v2: Val) -> Instr {
    Instr::ICmp(v1, v2)
}

/// Creates a conditional move instruction.
///
/// `comp_op` should be a condition-code operand such as `EQ.to_value()`.
pub fn cmov(comp_op: Val, v1: Val, v2: Val) -> Instr {
    Instr::ICMov(comp_op, v1, v2)
}

/// Creates a conditional jump instruction.
///
/// `cop` should be a condition-code operand, and `lbl` should be a label
/// operand.
pub fn ijump(cop: Val, lbl: Val) -> Instr {
    Instr::IJump(cop, lbl)
}

/// Creates an `and` instruction that bitwise-ands `v2` into `v1`.
pub fn iand(v1: Val, v2: Val) -> Instr {
    Instr::IAnd(v1, v2)
}

/// Creates an assembly label definition for `lbl`.
pub fn insert_label(lbl: &str) -> Instr {
    Instr::ILabel(Val::Label(lbl.to_string()))
}

/// Creates a `test` instruction comparing bits in `v1` and `v2`.
pub fn itest(v1: Val, v2: Val) -> Instr {
    Instr::ITest(v1, v2)
}

/// Creates an `xor` instruction that bitwise-xors `v2` into `v1`.
pub fn ixor(v1: Val, v2: Val) -> Instr {
    Instr::IXor(v1, v2)
}

/// Creates a `sar` instruction that arithmetically shifts `reg` right.
pub fn isar(reg: Val, shift_amount: Val) -> Instr {
    Instr::ISar(reg, shift_amount)
}

/// Creates a `lea` instruction that computes an address into `v1`.
pub fn ilea(v1: Val, v2: Val) -> Instr {
    Instr::ILea(v1, v2)
}

/// Creates an unconditional `jmp` instruction to `lbl`.
pub fn jmp(lbl: &str) -> Instr {
    Instr::IJmp(Val::Label(lbl.to_string()))
}

/// Creates a `call` instruction for the named function label.
pub fn icall(fname: &str) -> Instr {
    Instr::ICall(Val::Label(fname.to_string()))
}

/// Creates a `push` instruction.
///
/// At runtime, `push` subtracts 8 from `rsp` and stores `val` at `[rsp]`.
pub fn ipush(val: Val) -> Instr {
    Instr::IPush(val)
}

/// Creates a `pop` instruction.
///
/// At runtime, `pop` loads the value at `[rsp]` into `val` and adds 8 to
/// `rsp`.
pub fn ipop(val: Val) -> Instr {
    Instr::IPop(val)
}

/// Creates a raw hexadecimal-style operand from an integer mask.
///
/// This currently stores the decimal string form of `hex_val`, matching the
/// assembler operands expected by the rest of the compiler.
pub fn hex(hex_val: i32) -> Val {
    Val::Hex(hex_val.to_string())
}

impl Instr {
    /// Formats this instruction as x86-64 assembly text.
    pub fn to_string(&self) -> String {
        match &self {
            Instr::IRet => format!("ret"),
            Instr::IMov(v1, v2) => format!("mov {}, {}", v1.to_string(), v2.to_string()),
            Instr::IAdd(v1, v2) => format!("add {}, {}", v1.to_string(), v2.to_string()),
            Instr::ISub(v1, v2) => format!("sub {}, {}", v1.to_string(), v2.to_string()),
            Instr::IMul(v1, v2) => format!("imul {}, {}", v1.to_string(), v2.to_string()),
            Instr::ICmp(v1, v2) => format!("cmp {}, {}", v1.to_string(), v2.to_string()),
            Instr::ICMov(cop, v1, v2) => format!(
                "cmov{} {}, {}",
                cop.to_string(),
                v1.to_string(),
                v2.to_string()
            ),
            Instr::IJump(cop, lbl) => format!("j{} {}", cop.to_string(), lbl.to_string()),
            Instr::IAnd(v1, v2) => format!("and {}, {}", v1.to_string(), v2.to_string()),
            Instr::ILabel(lbl) => format!("{}:", lbl.to_string()),
            Instr::ITest(v1, v2) => format!("test {}, {}", v1.to_string(), v2.to_string()),
            Instr::IXor(v1, v2) => format!("xor {}, {}", v1.to_string(), v2.to_string()),
            Instr::ISar(reg, amount) => format!("sar {}, {}", reg.to_string(), amount.to_string()),
            Instr::IJmp(lbl) => format!("jmp {}", lbl.to_string()),
            Instr::ICall(fname) => format!("call {}", fname.to_string()),
            Instr::IPush(val) => format!("push {}", val.to_string()),
            Instr::IPop(val) => format!("pop {}", val.to_string()),
            Instr::IRaw(s) => s.to_string(),
            Instr::IShl(v1, v2) => format!("shl {}, {}", v1.to_string(), v2.to_string()),
            Instr::ILea(v1, v2) => format!("lea {}, {}", v1.to_string(), v2.to_string()),
        }
    }
}
