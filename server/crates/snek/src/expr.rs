use crate::instrs::*;
use crate::stack::Stack;
use core::panic;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum UnOp {
    Add1,
    Sub1,
    IsNum,
    IsBool,
    Print,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum BinOp {
    Plus,
    Minus,
    Times,
    Equal,
    Greater,
    GreaterEqual,
    Less,
    LessEqual,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Defn {
    pub name: String,
    pub params: Vec<String>,
    pub body: Expr,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct Prog {
    pub defns: Vec<Defn>,
    pub main: Expr,
}

#[derive(Clone, Copy)]
pub enum Type {
    Num,
    Bool,
    Vec,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Index {
    First,
    Last,
    I(i64),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Expr {
    Number(i64),
    Boolean(bool),
    Id(String),
    Input,
    Let(Vec<(String, Expr)>, Box<Expr>),
    UnOp(UnOp, Box<Expr>),
    BinOp(BinOp, Box<Expr>, Box<Expr>),
    Set(String, Box<Expr>),
    Block(Vec<Expr>),
    If(Box<Expr>, Box<Expr>, Box<Expr>),
    Loop(Box<Expr>),
    Break(Box<Expr>),
    Fn(String, Vec<Expr>),
    Nil,
    Vec(Vec<Expr>),
    VecGet(Box<Expr>, Index),
    VecLen(Box<Expr>),
    VecSet(Box<Expr>, Index, Box<Expr>),
}

pub fn num(n: i64) -> Expr {
    Expr::Number(n)
}

pub fn add1(e: Expr) -> Expr {
    Expr::UnOp(UnOp::Add1, Box::new(e))
}

pub fn sub1(e: Expr) -> Expr {
    Expr::UnOp(UnOp::Sub1, Box::new(e))
}

pub fn isnum(e: Expr) -> Expr {
    Expr::UnOp(UnOp::IsNum, Box::new(e))
}

pub fn isbool(e: Expr) -> Expr {
    Expr::UnOp(UnOp::IsBool, Box::new(e))
}

pub fn print(e: Expr) -> Expr {
    Expr::UnOp(UnOp::Print, Box::new(e))
}

pub fn plus(e1: Expr, e2: Expr) -> Expr {
    Expr::BinOp(BinOp::Plus, Box::new(e1), Box::new(e2))
}

pub fn minus(e1: Expr, e2: Expr) -> Expr {
    Expr::BinOp(BinOp::Minus, Box::new(e1), Box::new(e2))
}

pub fn times(e1: Expr, e2: Expr) -> Expr {
    Expr::BinOp(BinOp::Times, Box::new(e1), Box::new(e2))
}

pub fn lt(e1: Expr, e2: Expr) -> Expr {
    Expr::BinOp(BinOp::Less, Box::new(e1), Box::new(e2))
}

pub fn le(e1: Expr, e2: Expr) -> Expr {
    Expr::BinOp(BinOp::LessEqual, Box::new(e1), Box::new(e2))
}

pub fn gt(e1: Expr, e2: Expr) -> Expr {
    Expr::BinOp(BinOp::Greater, Box::new(e1), Box::new(e2))
}

pub fn ge(e1: Expr, e2: Expr) -> Expr {
    Expr::BinOp(BinOp::GreaterEqual, Box::new(e1), Box::new(e2))
}

pub fn eq(e1: Expr, e2: Expr) -> Expr {
    Expr::BinOp(BinOp::Equal, Box::new(e1), Box::new(e2))
}

fn number(n: i64) -> Vec<Instr> {
    vec![imov(RAX.to_value(), tagged(n))]
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum CompilerErr {
    Overflow,
    InvalidArg,
    InvalidCond,
    TypeMismatch,
    OutOfBounds,
    HeapOverflow,
}

impl CompilerErr {
    pub fn code(self) -> i64 {
        match self {
            CompilerErr::Overflow => 1,
            CompilerErr::InvalidArg => 2,
            CompilerErr::InvalidCond => 3,
            CompilerErr::TypeMismatch => 4,
            CompilerErr::OutOfBounds => 5,
            CompilerErr::HeapOverflow => 6,
        }
    }

    pub fn message(self) -> &'static str {
        match self {
            CompilerErr::Overflow => "overflow",
            CompilerErr::InvalidArg => "invalid argument; operand requires same types",
            CompilerErr::InvalidCond => "invalid condition; must be type bool",
            CompilerErr::TypeMismatch => "invalid argument, types are mismatched; types must match",
            CompilerErr::OutOfBounds => "index out of bounds",
            CompilerErr::HeapOverflow => "out of heap space",
        }
    }
}

impl From<CompilerErr> for i64 {
    fn from(err: CompilerErr) -> Self {
        err.code()
    }
}

impl From<CompilerErr> for &str {
    fn from(err: CompilerErr) -> Self {
        err.message()
    }
}

impl TryFrom<i64> for CompilerErr {
    type Error = ();
    fn try_from(errcode: i64) -> Result<Self, Self::Error> {
        match errcode {
            1 => Ok(CompilerErr::Overflow),
            2 => Ok(CompilerErr::InvalidArg),
            3 => Ok(CompilerErr::InvalidCond),
            4 => Ok(CompilerErr::TypeMismatch),
            5 => Ok(CompilerErr::OutOfBounds),
            _ => Err(()),
        }
    }
}

/// Checks the type stored in `rax` and throws compiler error if mismatched
/// Instructions to check the type of `rax`.
/// The code does NOT modify `rax`.
///
/// Equivalent to the asm code:
/// ```asm
/// mov rcx, rax
/// and rcx, 1
/// cmp rcx, {expected}
/// mov rdi, {err}
/// jne label_error"
/// ```
///
/// where `expected` is the first arg `ty`
fn check_type(ty: Type, err: CompilerErr) -> Vec<Instr> {
    let (expected, mask) = match ty {
        Type::Num => (0x0, 0x1),
        Type::Bool => (0x3, 0x3),
        Type::Vec => (0x1, 0x7),
    };

    let mut code = vec![];

    if let Type::Vec = ty {
        code.push(cmp(RAX.into(), NIL));
        code.push(imov(RDI.into(), untagged(err.into())));
        code.push(ijump(EQ.to_value(), Val::Label("label_error".to_string())));
    }

    code.extend(vec![
        imov(RCX.into(), RAX.into()),
        iand(RCX.into(), hex(mask)),
        cmp(RCX.into(), hex(expected)),
        imov(RDI.into(), untagged(err.into())),
        ijump(NE.to_value(), Val::Label("label_error".to_string())),
    ]);

    code
}

fn compare_operands(stack_pos: i64, err: CompilerErr) -> Vec<Instr> {
    let mut code: Vec<Instr> = vec![];
    code.push(imov(RCX.to_value(), RAX.to_value()));
    code.push(ixor(RCX.to_value(), offset(RBP, stack_pos)));
    code.push(itest(RCX.to_value(), untagged(1)));
    code.push(imov(RDI.to_value(), untagged(err.into())));
    code.push(ijump(NZ.to_value(), Val::Label("label_error".to_string())));
    code
}

/// Check if the idx is within the bounds of the vector
/// jumps to `CompilerErr::OutOfBounds` if the index is greater than the size of the vector.
/// Does not modify `rax`
fn check_idx_bounds(idx: i64) -> Vec<Instr> {
    let err = CompilerErr::OutOfBounds;
    let mut code = vec![];

    // n ? idx
    code.push(imov(RCX.into(), byte_offset(RAX, 0)));
    code.push(cmp(RCX.into(), Val::RawNum(idx)));
    code.push(imov(RDI.to_value(), untagged(err.into())));
    code.push(ijump(L.to_value(), Val::Label("label_error".to_string())));
    // if n < idx -> error
    code
}

fn check_idx_upper_bound(idx: i64) -> Vec<Instr> {
    let err = CompilerErr::OutOfBounds;
    let mut code = vec![];

    code.push(imov(RCX.into(), byte_offset(RAX, 0)));
    code.push(cmp(RCX.into(), Val::RawNum(idx)));
    code.push(imov(RDI.to_value(), untagged(err.into())));
    code.push(ijump(LE.to_value(), Val::Label("label_error".to_string())));
    code
}

fn compare(op: Val, pos: i64) -> Vec<Instr> {
    let mut code: Vec<Instr> = vec![];

    /*
    (? a b) => a ? b =>
    a gets pushed into [rbp - 8 * pos]
    b gets pushed into rax

    cmp [rbp - 8 * pos], rax
    mov rcx, 1 ; false
    mov rdx, 3 ; true
    cmov? rax, rdx ; if a ? b then rax gets true else false.

    where ? can be any comparison operator.
    */

    code.push(cmp(offset(RBP, pos), RAX.to_value()));
    code.push(imov(RAX.to_value(), FALSE));
    code.push(imov(RDX.to_value(), TRUE));
    code.push(cmov(op, RAX.to_value(), RDX.to_value()));
    code
}

fn check_isnum() -> Vec<Instr> {
    let mut code: Vec<Instr> = vec![];
    code.push(itest(RAX.to_value(), untagged(1)));
    code.push(imov(RAX.to_value(), FALSE));
    code.push(imov(RBX.to_value(), TRUE));
    code.push(cmov(Z.to_value(), RAX.to_value(), RBX.to_value()));
    code
}

fn check_isbool() -> Vec<Instr> {
    let mut code: Vec<Instr> = vec![];
    code.push(itest(RAX.to_value(), untagged(1)));
    code.push(imov(RAX.to_value(), FALSE));
    code.push(imov(RBX.to_value(), TRUE));
    code.push(cmov(NZ.to_value(), RAX.to_value(), RBX.to_value()));
    code
}

/// Tries to allocate memory
/// Throws error if the heap is out of memory
/// If successful, the function call modifies `r11` by increasing the value by `size` effectively moving it to the next pointer head.
fn try_malloc(size: i64) -> Vec<Instr> {
    let mut code = vec![];
    let err = CompilerErr::HeapOverflow;

    code.push(ilea(RDX.into(), byte_offset(R11, size)));
    code.push(cmp(RDX.into(), R15.into()));
    code.push(imov(RDI.to_value(), untagged(err.into())));
    code.push(Instr::IRaw(format!("ja label_error")));
    code.push(iadd(R11.into(), untagged(size)));
    /*
    lea rdx, [r11 + size]
    cmp rdx, r15
    ja out_of_memory_error

    add r11, size
    */
    code
}

pub struct Ctxt {
    counter: usize,
    break_labels: Vec<String>,
}

impl Ctxt {
    pub fn new() -> Ctxt {
        Self {
            counter: 0,
            break_labels: vec![],
        }
    }

    fn new_lbl(&mut self) -> usize {
        let next = self.counter;
        self.counter += 1;
        next
    }

    fn curr_break_lbl(&self) -> &str {
        self.break_labels
            .last()
            .expect("error: break outside of loop.")
    }

    pub fn push_exit_lbl(&mut self, lbl: &str) {
        self.break_labels.push(lbl.to_string())
    }

    pub fn compile_setup(&self, e: &Expr) -> Vec<Instr> {
        let vars = max_vars(e);
        /*
        push rbp
        mov rbp, rsp
        sub rbp, {vars} * 8
        */
        vec![
            ipush(RBP.into()),
            imov(RBP.into(), RSP.into()),
            isub(RSP.into(), Val::RawNum(vars * 8)),
        ]
    }

    pub fn compile_teardown(&self) -> Vec<Instr> {
        /*
        mov rsp, rbp
        pop rbp
        ret
        */
        vec![imov(RSP.into(), RBP.into()), ipop(RBP.into()), Instr::IRet]
    }

    pub fn compile_prog(&mut self, prog: &Prog, exit_lbl: &str) -> Vec<Instr> {
        let mut setup = self.compile_setup(&prog.main);
        let mut teardown = self.compile_teardown();
        let mut main_code = self.compile_to_instrs(&prog.main, &Stack::new(), true);
        let mut defs_code = prog
            .defns
            .iter()
            .flat_map(|d| self.compile_defn(d))
            .collect::<Vec<_>>();

        let mut code = vec![
            insert_label("label_error"),
            ipush(RSP.into()),
            icall("snek_error"),
            insert_label("our_code_starts_here"),
        ];

        code.append(&mut setup);
        // move heap ptr into r11
        code.push(imov(R11.into(), RSI.into()));
        // move end heap into r15
        code.push(ilea(R15.into(), byte_offset(RSI, 800000)));
        code.push(imov(R11.into(), RSI.into()));
        code.append(&mut main_code);
        code.push(insert_label(exit_lbl));
        code.append(&mut teardown);
        code.append(&mut defs_code);
        code
    }

    pub fn compile_defn(&mut self, defn: &Defn) -> Vec<Instr> {
        let mut setup = self.compile_setup(&defn.body);
        let mut body = self.compile_to_instrs(&defn.body, &Stack::params(&defn.params), true);
        let mut teardown = self.compile_teardown();
        let name = &defn.name;

        /*
        {name}
        {setup}
        {name}_body:  <-- new; tail call jump to location.
        {body}
        {teardown}
        */

        let mut code = vec![];
        code.push(insert_label(name));
        code.append(&mut setup);
        code.push(insert_label(format!("{name}_body").as_str()));
        code.append(&mut body);
        code.append(&mut teardown);
        code
    }

    pub fn compile_to_instrs(&mut self, e: &Expr, stack: &Stack, tail_call: bool) -> Vec<Instr> {
        match e {
            Expr::Number(n) => number(*n),
            Expr::Boolean(b) => match b {
                true => vec![imov(RAX.to_value(), TRUE)],   // 0b11
                false => vec![imov(RAX.to_value(), FALSE)], // 0b01
            },

            Expr::Id(s) => {
                let s_pos = match stack.get(s) {
                    Some(s_pos) => s_pos,
                    None => panic!("Unbound variable identifier {s}"),
                };

                vec![imov(RAX.to_value(), offset(RBP, s_pos))]
            }

            Expr::Input => vec![imov(RAX.to_value(), RDI.to_value())],

            Expr::Let(items, expr) => {
                let mut instrs: Vec<Instr> = Vec::new();
                let mut cur_stack = stack.clone();
                let mut bindings = Vec::new();

                for (s, e) in items {
                    // Duplicate names are only an error within this binding list.
                    // Nested lets may shadow outer bindings.
                    if bindings.contains(s) {
                        panic!("Duplicate binding")
                    }
                    bindings.push(s.to_string());

                    let mut rhs_instr = self.compile_to_instrs(e, &cur_stack, false);
                    instrs.append(&mut rhs_instr);

                    let (s_pos, next_stack) = cur_stack.push(s.to_string());

                    instrs.push(imov(offset(RBP, s_pos), RAX.to_value()));

                    cur_stack = next_stack;
                }

                let mut body_instr = self.compile_to_instrs(expr, &cur_stack, tail_call);
                instrs.append(&mut body_instr);
                return instrs;
            }

            Expr::UnOp(op1, expr) => match op1 {
                UnOp::Add1 => {
                    let mut code = self.compile_to_instrs(expr, stack, false);
                    code.append(&mut check_type(Type::Num, CompilerErr::InvalidArg));
                    code.push(iadd(RAX.to_value(), tagged(1)));
                    code.push(imov(RDI.to_value(), untagged(CompilerErr::Overflow.code())));
                    code.push(ijump(O.to_value(), Val::Label("label_error".to_string())));

                    code
                }

                UnOp::Sub1 => {
                    let mut code = self.compile_to_instrs(expr, stack, false);
                    code.append(&mut check_type(Type::Num, CompilerErr::InvalidArg));
                    code.push(isub(RAX.to_value(), tagged(1)));
                    code.push(imov(RDI.to_value(), untagged(CompilerErr::Overflow.code())));
                    code.push(ijump(O.to_value(), Val::Label("label_error".to_string())));
                    code
                }

                UnOp::IsNum => {
                    let mut code = self.compile_to_instrs(expr, stack, false);
                    code.append(&mut check_isnum());
                    code
                }

                UnOp::IsBool => {
                    let mut code = self.compile_to_instrs(expr, stack, false);
                    code.append(&mut check_isbool());
                    code
                }
                UnOp::Print => {
                    let mut code = self.compile_to_instrs(expr, stack, false);
                    /*
                    {e}
                    mov rdi, rax
                    push r11
                    call snek_println
                    pop r11
                    */

                    code.push(imov(RDI.into(), RAX.into()));
                    code.push(ipush(R11.into()));
                    code.push(icall("snek_print"));
                    code.push(ipop(R11.into()));
                    code
                }
            },

            Expr::BinOp(op2, e1, e2) => match op2 {
                BinOp::Plus => {
                    let mut code = self.compile_to_instrs(e1, stack, false);
                    let (tmp_pos, stack1) = stack.push("#@#@".to_string());
                    let mut e2_instr = self.compile_to_instrs(e2, &stack1, false);

                    code.append(&mut check_type(Type::Num, CompilerErr::InvalidArg));
                    // push rax onto the stack
                    code.push(imov(offset(RBP, tmp_pos), RAX.to_value()));
                    code.append(&mut e2_instr);

                    code.append(&mut check_type(Type::Num, CompilerErr::InvalidArg));
                    // go with op
                    code.push(iadd(RAX.to_value(), offset(RBP, tmp_pos)));
                    // check overflow
                    // (2a) + (2b) = 2(a+b)
                    code.push(imov(RDI.to_value(), untagged(CompilerErr::Overflow.code())));
                    code.push(ijump(O.to_value(), Val::Label("label_error".to_string())));

                    code
                }

                BinOp::Minus => {
                    let mut code = self.compile_to_instrs(e1, stack, false);
                    let (tmp_pos, new_stack) = stack.push("#@#@".to_string());
                    let mut e2_instr = self.compile_to_instrs(e2, &new_stack, false);

                    // first arg1
                    code.append(&mut check_type(Type::Num, CompilerErr::InvalidArg));
                    // push onto stack
                    code.push(imov(offset(RBP, tmp_pos), RAX.to_value()));
                    code.append(&mut e2_instr);
                    // check arg2
                    code.append(&mut check_type(Type::Num, CompilerErr::InvalidArg));
                    // perform op
                    code.push(isub(offset(RBP, tmp_pos), RAX.to_value()));

                    // check overflow
                    code.push(imov(RDI.to_value(), untagged(CompilerErr::Overflow.code())));
                    code.push(ijump(O.to_value(), Val::Label("label_error".to_string())));
                    code.push(imov(RAX.to_value(), offset(RBP, tmp_pos)));

                    code
                }

                BinOp::Times => {
                    let mut code = self.compile_to_instrs(e1, stack, false);
                    let (tmp_pos, new_stack) = stack.push("#@#@".to_string());
                    let mut e2_instr = self.compile_to_instrs(e2, &new_stack, false);

                    // check arg1 type
                    code.append(&mut check_type(Type::Num, CompilerErr::InvalidArg));
                    // push rax onto stack
                    code.push(imov(offset(RBP, tmp_pos), RAX.to_value()));
                    code.append(&mut e2_instr);
                    // check arg2 type
                    code.append(&mut check_type(Type::Num, CompilerErr::InvalidArg));

                    code.push(isar(RAX.to_value(), untagged(1)));
                    code.push(imul(RAX.to_value(), offset(RBP, tmp_pos)));

                    // (2a) * (2b) = 4ab => 4ab / 2 == 4ab >> 1 = 2ab
                    // so, apply shift by 1
                    /*
                    sar rax, 1
                    imul rax, [tmp]
                    jo label_error
                    */
                    code.push(imov(RDI.to_value(), untagged(1)));
                    code.push(ijump(O.to_value(), Val::Label("label_error".to_string())));
                    code
                }

                BinOp::Equal => {
                    let mut code = self.compile_to_instrs(e1, stack, false);
                    let (tmp, stack1) = stack.push("#@#@".to_string());
                    let mut e2_instr = self.compile_to_instrs(e2, &stack1, false);

                    /*
                    load both args;
                    bit flip both args
                    if they have the same value, then same type
                    o.w.
                    throw an error
                    */

                    code.push(imov(offset(RBP, tmp), RAX.to_value()));

                    code.append(&mut e2_instr);

                    /*
                    mov rcx, rax
                    xor rcx, [tmp]
                    test rcx, 1
                    jnz label
                    */

                    code.append(&mut compare_operands(tmp, CompilerErr::TypeMismatch));
                    code.append(&mut compare(EQ.to_value(), tmp));

                    code
                }
                BinOp::Greater => {
                    let mut code = self.compile_to_instrs(e1, stack, false);
                    let (tmp, stack1) = stack.push("#@#@".to_string());
                    let mut e2_code = self.compile_to_instrs(e2, &stack1, false);

                    code.append(&mut check_type(Type::Num, CompilerErr::InvalidArg));
                    code.push(imov(offset(RBP, tmp), RAX.to_value()));
                    code.append(&mut e2_code);
                    code.append(&mut check_type(Type::Num, CompilerErr::InvalidArg));

                    code.append(&mut compare(G.to_value(), tmp));
                    code
                }
                BinOp::GreaterEqual => {
                    let mut code = self.compile_to_instrs(e1, stack, false);
                    let (tmp, stack1) = stack.push("#@#@".to_string());
                    let mut e2_code = self.compile_to_instrs(e2, &stack1, false);

                    code.append(&mut check_type(Type::Num, CompilerErr::InvalidArg));
                    code.push(imov(offset(RBP, tmp), RAX.to_value()));
                    code.append(&mut e2_code);
                    code.append(&mut check_type(Type::Num, CompilerErr::InvalidArg));

                    code.append(&mut compare(GE.to_value(), tmp));
                    code
                }
                BinOp::Less => {
                    let mut code = self.compile_to_instrs(e1, stack, false);
                    let (tmp, stack1) = stack.push("#@#@".to_string());
                    let mut e2_code = self.compile_to_instrs(e2, &stack1, false);

                    code.append(&mut check_type(Type::Num, CompilerErr::InvalidArg));
                    code.push(imov(offset(RBP, tmp), RAX.to_value()));
                    code.append(&mut e2_code);
                    code.append(&mut check_type(Type::Num, CompilerErr::InvalidArg));

                    code.append(&mut compare(L.to_value(), tmp));
                    code
                }
                BinOp::LessEqual => {
                    let mut code = self.compile_to_instrs(e1, stack, false);
                    let (tmp, stack1) = stack.push("#@#@".to_string());
                    let mut e2_code = self.compile_to_instrs(e2, &stack1, false);

                    code.append(&mut check_type(Type::Num, CompilerErr::InvalidArg));
                    code.push(imov(offset(RBP, tmp), RAX.to_value()));
                    code.append(&mut e2_code);
                    code.append(&mut check_type(Type::Num, CompilerErr::InvalidArg));
                    code.append(&mut compare(LE.to_value(), tmp));
                    code
                }
            },

            Expr::If(cond, e1, e2) => {
                let mut code = self.compile_to_instrs(cond, stack, false);
                let mut then_code = self.compile_to_instrs(e1, stack, tail_call);
                let mut else_code = self.compile_to_instrs(e2, stack, tail_call);

                let if_false = format!("if_false_{}", i32::try_from(self.new_lbl()).unwrap());

                code.append(&mut check_type(Type::Bool, CompilerErr::InvalidCond));
                code.push(cmp(RAX.to_value(), FALSE));
                code.push(ijump(EQ.to_value(), Val::Label(if_false.to_string())));

                let if_true = format!("if_true_{}", i32::try_from(self.new_lbl()).unwrap());

                code.push(insert_label(&if_true));
                code.append(&mut then_code);

                let if_exit = format!("if_exit_{}", i32::try_from(self.new_lbl()).unwrap());

                code.push(jmp(&if_exit));
                code.push(insert_label(&if_false));
                code.append(&mut else_code);
                code.push(insert_label(&if_exit));

                code

                // push all onto result.
                /*
                {cond_code}
                {cond_test}
                cmp rax, FALSE
                je #if_false
                #if_true:
                    {then_code}
                    jmp #if_exit
                #if_false:
                    {else_cdoe}
                #if_exit:
                */
            }
            Expr::Loop(body) => {
                let mut code: Vec<Instr> = vec![];
                let lbl = self.new_lbl();
                let start_lbl = format!("loop_start_{lbl}");
                let exit_lbl = format!("loop_exit_{lbl}");

                self.break_labels.push(exit_lbl.clone());
                let mut body_code = self.compile_to_instrs(body, stack, false);
                self.break_labels.pop();

                code.push(insert_label(&start_lbl));
                code.append(&mut body_code);
                code.push(jmp(&start_lbl));
                code.push(insert_label(&exit_lbl));

                /*
                {start_lbl}
                {body}
                jmp {start_lbl}
                {exit_lbl}
                */
                code
            }
            Expr::Break(e) => {
                let exit_lbl = self.curr_break_lbl().to_string();
                let mut e_code = self.compile_to_instrs(e, stack, false);
                e_code.push(jmp(&exit_lbl));
                e_code
                /*
                {e_code}
                jmp {exit_lbl}
                */
            }

            Expr::Set(x, expr) => {
                let x_pos = match stack.get(x) {
                    Some(x_pos) => x_pos,
                    None => panic!("Unbound variable identifier {x}"),
                };
                let mut expr_code = self.compile_to_instrs(expr, stack, false);

                expr_code.push(imov(offset(RBP, x_pos), RAX.to_value()));
                expr_code
            }

            Expr::Block(exprs) => {
                let n = exprs.len();
                exprs
                    .iter()
                    .enumerate()
                    .flat_map(|(i, e)| self.compile_to_instrs(e, stack, (i == n - 1) && tail_call))
                    .collect::<Vec<_>>()
            }

            Expr::Fn(name, exprs) => {
                let mut code = vec![];
                let n = exprs.len();
                let mut cur_stack = stack.clone();
                let mut arg_slots = vec![];
                let (next_slot, _) = stack.push("#@#@".to_string());
                let live_slots = next_slot - 1;

                /*
                Call(f, e1, e2, ..., en)

                First, evaluate args left-to-right and save each result in a
                fresh compiler temp slot below rbp. The stack passed to later
                args includes earlier temp slots, so nested calls know what
                local slots they must preserve.

                    {e1_code}
                    mov [rbp - 8 * t1], rax
                    {e2_code}
                    mov [rbp - 8 * t2], rax
                    ...
                    {en_code}
                    mov [rbp - 8 * tn], rax

                If this is not a tail call, push the staged args in reverse
                order. Before pushing, move rsp below the deepest compiler temp
                so pushes/call do not overwrite caller locals such as binary-op
                temps.

                    mov rsp, rbp
                    sub rsp, 8 * tn
                    mov rax, [rbp - 8 * tn]
                    push rax
                    ...
                    mov rax, [rbp - 8 * t1]
                    push rax
                    call f
                    add rsp, 8 * n

                If this is a tail call, copy staged args into the current
                frame's outgoing argument slots, restore rsp/rbp to the shape
                expected at function entry, and jump to f. Jumping to f instead
                of f_body lets mutually recursive functions run their own setup.

                    mov rax, [rbp - 8 * t1]
                    mov [rbp + 8 * 2], rax
                    mov rax, [rbp - 8 * t2]
                    mov [rbp + 8 * 3], rax
                    ...
                    mov rbx, [rbp]      ; caller's rbp
                    mov rsp, rbp
                    add rsp, 8          ; original return address is now on top
                    mov rbp, rbx
                    jmp f
                */

                for e in exprs {
                    let (arg_slot, next_stack) = cur_stack.push("#@#@".to_string());
                    let mut e_code = self.compile_to_instrs(e, &cur_stack, false);
                    code.append(&mut e_code);
                    code.push(imov(offset(RBP, arg_slot), RAX.into()));
                    arg_slots.push(arg_slot);
                    cur_stack = next_stack;
                }

                if tail_call {
                    for (i, arg_slot) in arg_slots.iter().enumerate() {
                        code.push(imov(RAX.into(), offset(RBP, *arg_slot)));
                        code.push(imov(offset(RBP, -((i as i64) + 2)), RAX.into()));
                    }

                    code.push(imov(RBX.into(), offset(RBP, 0)));
                    code.push(imov(RSP.into(), RBP.into()));
                    code.push(iadd(RSP.into(), Val::RawNum(8)));
                    code.push(imov(RBP.into(), RBX.into()));
                    code.push(jmp(name));
                } else {
                    let max_slot = arg_slots.last().copied().unwrap_or(live_slots);
                    if max_slot > 0 {
                        code.push(imov(RSP.into(), RBP.into()));
                        code.push(isub(RSP.into(), Val::RawNum(max_slot * 8)));
                    }
                    for arg_slot in arg_slots.iter().rev() {
                        code.push(imov(RAX.into(), offset(RBP, *arg_slot)));
                        code.push(ipush(RAX.into()));
                    }
                    code.push(icall(name));
                    if n > 0 {
                        code.push(iadd(RSP.into(), Val::RawNum(n as i64 * 8)));
                    }
                }

                code
            }
            Expr::Nil => vec![imov(RAX.into(), NIL)],

            Expr::Vec(exprs) => {
                let n = exprs.len() as i64;
                let size = 8 * (n + 1);
                let (base_slot, mut cur_stack) = stack.push("#@#@#".to_string());
                let mut code = vec![];

                // base
                code.push(imov(R10.into(), R11.into()));
                code.push(imov(offset(RBP.into(), base_slot), R10.into()));
                // reserve
                code.append(&mut try_malloc(size));
                // size
                code.push(imov(byte_offset(R10.into(), 0), untagged(n)));

                for (i, e) in exprs.iter().enumerate() {
                    let i = i as i64;
                    let (_, new_stack) = stack.push("#@#@#".to_string());
                    let mut e_code = self.compile_to_instrs(e, &cur_stack, false);
                    code.append(&mut e_code);

                    code.push(imov(R10.into(), offset(RBP, base_slot)));
                    code.push(imov(byte_offset(R10.into(), 8 * (i + 1)), RAX.into()));
                    cur_stack = new_stack;
                }

                code.push(imov(R10.into(), offset(RBP, base_slot)));
                code.push(imov(RAX.into(), R10.into()));
                code.push(iadd(RAX.into(), untagged(1)));

                code
            }
            Expr::VecGet(e, idx) => {
                let mut code = vec![];
                let mut e_code = self.compile_to_instrs(e, stack, false);
                // e_code must be a vec.
                // if not compile error. type error.

                code.append(&mut e_code);
                code.append(&mut check_type(Type::Vec, CompilerErr::InvalidArg));
                code.push(isub(RAX.into(), untagged(1)));

                match idx {
                    Index::First => {
                        code.append(&mut check_idx_upper_bound(0));
                        code.push(imov(RAX.into(), byte_offset(RAX, 8)));
                    }
                    Index::Last => {
                        code.append(&mut check_idx_upper_bound(0));
                        code.push(imov(RCX.into(), byte_offset(RAX, 0)));
                        code.push(Instr::IRaw("lea rax, [rax + 8 * rcx]".to_string()));
                        code.push(imov(RAX.into(), byte_offset(RAX, 0)));
                    }
                    Index::I(i) => {
                        code.append(&mut check_idx_bounds(i.abs()));
                        if *i < 0 {
                            /*
                            mov rcx, [rax] ;; get the length (untagged)
                            add rcx, i ;; n - i
                            lea rax, [rax + 8 * rcx]
                            mov rax, [rax] ;; mem addr
                            */
                            code.push(imov(RCX.into(), byte_offset(RAX, 0)));
                            code.push(iadd(RCX.into(), untagged(*i)));
                            code.push(iadd(RCX.into(), untagged(1)));
                            code.push(Instr::IRaw("lea rax, [rax + 8 * rcx]".to_string()));
                            code.push(imov(RAX.into(), byte_offset(RAX, 0)));
                        } else if *i > 0 {
                            code.append(&mut check_idx_upper_bound(*i));
                            let idx = 8 * (i + 1);
                            code.push(imov(RAX.into(), byte_offset(RAX, idx)));
                        }
                    }
                };
                code
            }
            Expr::VecLen(expr) => {
                let mut code = vec![];
                let mut e_code = self.compile_to_instrs(expr, stack, false);

                code.append(&mut e_code);
                code.append(&mut check_type(Type::Vec, CompilerErr::InvalidArg));
                code.push(isub(RAX.into(), untagged(1))); // untag the vector
                code.push(imov(RCX.into(), byte_offset(RAX, 0))); // then get the length
                code.push(Instr::IShl(RCX.into(), untagged(1)));
                code.push(imov(RAX.into(), RCX.into()));
                /*
                {e_code}
                ;; check e is a vector
                ;; untag vector
                mov rcx, [rax]
                shl rcx, 1 ;; tag the number
                mov rax, rcx
                */
                code
            }
            Expr::VecSet(vec_expr, idx, val) => {
                let mut code = vec![];
                let (tmp_pos, new_stack) = stack.push("#@#@#@".to_string());
                let mut vec_code = self.compile_to_instrs(vec_expr, &new_stack, false);
                let mut val_code = self.compile_to_instrs(val, &new_stack, false);

                code.append(&mut vec_code);
                code.append(&mut check_type(Type::Vec, CompilerErr::InvalidArg));
                code.push(imov(offset(RBP, tmp_pos), RAX.into())); // move the mem addr into {tmp} for later use.

                code.append(&mut val_code); // compile val code into rax

                // use the index and set the value
                // ensure the idx is in bounds tho.

                // load the val into rbx for later
                code.push(imov(RBX.into(), RAX.into()));
                // load ptr from stack into rax
                code.push(imov(RAX.into(), offset(RBP, tmp_pos)));
                code.push(isub(RAX.into(), untagged(1))); // remove the tag so we can do mem arthimetic

                match idx {
                    Index::First => {
                        code.append(&mut check_idx_upper_bound(0));
                        code.push(imov(byte_offset(RAX, 8), RBX.into()));
                    }
                    Index::Last => {
                        code.append(&mut check_idx_upper_bound(0));
                        // get the length of the vector
                        code.push(imov(RCX.into(), byte_offset(RAX, 0)));
                        // place the value of rbx into the last value
                        code.push(Instr::IRaw("mov [rax + 8 * rcx], rbx".to_string()));
                    }
                    Index::I(i) => {
                        code.append(&mut check_idx_bounds(i.abs()));
                        if *i < 0 {
                            // get the length
                            code.push(imov(RCX.into(), byte_offset(RAX, 0)));
                            // get the offset
                            code.push(iadd(RCX.into(), untagged(*i)));
                            code.push(iadd(RCX.into(), untagged(1)));
                            // load rbx into the index
                            code.push(Instr::IRaw("mov [rax + 8 * rcx], rbx".to_string()));
                        } else if *i > 0 {
                            code.append(&mut check_idx_upper_bound(*i));
                            let idx = 8 * (i + 1);
                            // load rbx straight into the mem addr
                            code.push(imov(byte_offset(RAX, idx), RBX.into()));
                        }
                    }
                };

                // return the pointer of the vector

                /*
                {vec_code}
                {check_vec_code}
                ;; ptr lives in rax
                mov [rbp - 8], rax            ;; stash ptr
                {val_code}                    ;; val lives in rax
                mov rbx, rax                  ;; stash val
                mov rax, [rbp - 8]            ;; ptr head in rax
                sub rax, 1                    ;; untag to use with mem addr
                {idx_logic_code}
                {if first}
                    mov [rax + 8], rbx        ;; move val into first slot
                {if last}
                    mov rcx, [rax]            ;; get the length
                    mov [rax + 8 * rcx], rbx  ;; place val into the last slot
                {if i < 0}
                    mov rcx, [rax]            ;; get the length
                    add rcx, i                ;; create the index offset from the last element
                    mov [rax + 8*rcx], rbx    ;; place val into slot
                {if i > 0}
                    mov [rax + 8*i], rbx      ;; place val into index slot
                add rax, 1                    ;; add ptr tag back
                */

                // add the ptr tag back
                code.push(iadd(RAX.into(), untagged(1)));
                code
            }
        }
    }
}

fn max_vars(e: &Expr) -> i64 {
    frame_slots(max_stack_slots(e, 0))
}

fn max_stack_slots(e: &Expr, env_size: i64) -> i64 {
    match e {
        Expr::Number(_) | Expr::Boolean(_) | Expr::Id(_) | Expr::Input | Expr::Nil => env_size,

        Expr::UnOp(_, expr)
        | Expr::Set(_, expr)
        | Expr::Loop(expr)
        | Expr::Break(expr)
        | Expr::VecGet(expr, _)
        | Expr::VecLen(expr) => max_stack_slots(expr, env_size),

        Expr::Block(exprs) => exprs
            .iter()
            .map(|expr| max_stack_slots(expr, env_size))
            .max()
            .unwrap_or(env_size),

        Expr::Vec(exprs) => {
            let element_env = env_size + 1;
            exprs
                .iter()
                .map(|expr| max_stack_slots(expr, element_env))
                .max()
                .unwrap_or(element_env)
                .max(element_env)
        }

        Expr::Fn(_, exprs) => {
            let mut cur_env = env_size;
            let mut max_slots = env_size;

            for expr in exprs {
                max_slots = max_slots.max(max_stack_slots(expr, cur_env));
                cur_env += 1;
                max_slots = max_slots.max(cur_env);
            }

            max_slots
        }

        Expr::BinOp(_, e1, e2) | Expr::VecSet(e1, _, e2) => {
            let rhs_env = env_size + 1;
            max_stack_slots(e1, env_size)
                .max(rhs_env)
                .max(max_stack_slots(e2, rhs_env))
        }

        Expr::If(e1, e2, e3) => max_stack_slots(e1, env_size)
            .max(max_stack_slots(e2, env_size))
            .max(max_stack_slots(e3, env_size)),

        Expr::Let(items, expr) => {
            let mut cur_env = env_size;
            let mut max_slots = env_size;

            for (_, rhs) in items {
                max_slots = max_slots.max(max_stack_slots(rhs, cur_env));
                cur_env += 1;
                max_slots = max_slots.max(cur_env);
            }

            max_slots.max(max_stack_slots(expr, cur_env))
        }
    }
}

fn frame_slots(max_slots: i64) -> i64 {
    let slots = max_slots.max(1);
    if slots % 2 == 0 {
        slots + 1
    } else {
        slots
    }
}

// input: expression
// output: assembly string code
// pub fn compile(e: &Expr) -> String {
//     let mut ctxt = Ctxt::new();
//     let instrs = ctxt.compile_to_instrs(e, &Stack::new());
//     let result = instrs.iter()
//         .map(Instr::to_string)
//         .collect::<Vec<_>>()
//         .join("\n");
//     return result
// }
