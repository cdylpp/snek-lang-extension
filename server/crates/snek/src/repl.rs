use crate::expr::{BinOp, Defn, Expr, Index, Prog, UnOp};
use crate::parser;
use im::HashMap;
use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use std::cell::RefCell;
use std::fmt;
use std::io::{self, Write};
use std::panic;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

const BANNER_WIDTH: usize = 72;
const BANNER_HEIGHT: usize = 7;

pub fn repl() {
    println!("{}", snek_banner());

    let mut buffer = String::new();
    let mut balance = 0_i64;

    loop {
        print!("rust> ");
        io::stdout().flush().expect("failed to flush stdout");

        let mut line = String::new();
        let bytes_read = io::stdin()
            .read_line(&mut line)
            .expect("failed to read line");

        if bytes_read == 0 {
            println!();
            break;
        }

        balance += paren_delta(&line);
        buffer.push_str(&line);

        if balance < 0 {
            println!("Error: unexpected closing parenthesis");
            buffer.clear();
            balance = 0;
            continue;
        }

        if balance > 0 {
            continue;
        }

        let input = buffer.trim();
        if !input.is_empty() {
            match eval_program(input) {
                Ok(v) => println!("{v}"),
                Err(err) => println!("Error: {err}"),
            }
        }

        buffer.clear();
    }
}

pub fn snek_banner() -> String {
    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    let mut rng = StdRng::seed_from_u64(seed);

    let phase1 = rng.gen_range(0.0..std::f64::consts::TAU);
    let phase2 = rng.gen_range(0.0..std::f64::consts::TAU);
    let phase3 = rng.gen_range(0.0..std::f64::consts::TAU);

    let snake_y = rng.gen_range(4..BANNER_HEIGHT);
    let snake_x = rng.gen_range(4..BANNER_WIDTH - 8);
    let snake = if rng.gen_bool(0.5) {
        "~<:3)~~"
    } else {
        "~~(ε:>~"
    };

    let mut grid = vec![vec![' '; BANNER_WIDTH]; BANNER_HEIGHT];

    for y in 0..BANNER_HEIGHT {
        for x in 0..BANNER_WIDTH {
            let xf = x as f64 / BANNER_WIDTH as f64;
            let yf = y as f64 / BANNER_HEIGHT as f64;

            let v = (xf * 38.0 + phase1).sin()
                + ((xf * 18.0 + yf * 12.0) + phase2).sin() * 0.8
                + ((xf * 9.0 - yf * 20.0) + phase3).cos() * 0.6;

            let contour = ((v * 3.0).round() as i32).abs();

            grid[y][x] = match contour {
                0 => ' ',
                1 => '.',
                2 => '~',
                3 => '-',
                4 => '=',
                _ => '∿',
            };
        }
    }

    let title = " S N E K   R E P L ";
    let title_x = (BANNER_WIDTH - title.len()) / 2;
    let title_y = 2;

    for (i, ch) in title.chars().enumerate() {
        grid[title_y][title_x + i] = ch;
    }

    for (i, ch) in snake.chars().enumerate() {
        if snake_x + i < BANNER_WIDTH {
            grid[snake_y][snake_x + i] = ch;
        }
    }

    grid.into_iter()
        .map(|row| row.into_iter().collect::<String>())
        .collect::<Vec<_>>()
        .join("\n")
}

fn paren_delta(s: &str) -> i64 {
    s.chars().fold(0, |delta, ch| match ch {
        '(' => delta + 1,
        ')' => delta - 1,
        _ => delta,
    })
}

fn eval_program(input: &str) -> Result<Val, Err> {
    let prog = panic::catch_unwind(|| parser::parse(input))
        .map_err(|_| Err::ParseError("invalid s-expression".to_string()))?;
    eval_prog(&prog)
}

fn eval_prog(prog: &Prog) -> Result<Val, Err> {
    let funs = prog
        .defns
        .iter()
        .map(|d| (d.name.clone(), d.clone()))
        .collect();
    let mut env = HashMap::new();
    eval(&prog.main, &mut env, &funs)
}

// interpreter
type Env = HashMap<String, Val>;
type FunEnv = HashMap<String, Defn>;

#[derive(Debug, Clone)]
pub enum Err {
    Break(Val),
    ParseError(String),
    TypeError(String),
    UnboundVar(String),
    Unsupported(String),
}

impl fmt::Display for Err {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Err::Break(v) => write!(f, "break outside loop with value {v}"),
            Err::ParseError(msg) => write!(f, "{msg}"),
            Err::TypeError(msg) => write!(f, "{msg}"),
            Err::UnboundVar(x) => write!(f, "unbound variable `{x}`"),
            Err::Unsupported(msg) => write!(f, "{msg}"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum Val {
    Num(i64),
    Bool(bool),
    Vector(Rc<RefCell<Vec<Val>>>),
}

impl PartialEq for Val {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Val::Num(lhs), Val::Num(rhs)) => lhs == rhs,
            (Val::Bool(lhs), Val::Bool(rhs)) => lhs == rhs,
            (Val::Vector(lhs), Val::Vector(rhs)) => *lhs.borrow() == *rhs.borrow(),
            _ => false,
        }
    }
}

impl Eq for Val {}

impl fmt::Display for Val {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Val::Num(n) => write!(f, "{n}"),
            Val::Bool(b) => write!(f, "{b}"),
            Val::Vector(vals) => {
                write!(f, "(vec")?;
                for val in vals.borrow().iter() {
                    write!(f, " {val}")?;
                }
                write!(f, ")")
            }
        }
    }
}

fn eval_id(x: &String, env: &Env) -> Result<Val, Err> {
    match env.get(x) {
        Some(v) => Ok(v.clone()),
        None => Err(Err::UnboundVar(x.clone())),
    }
}

fn eval_add1(v: Val) -> Result<Val, Err> {
    match v {
        Val::Num(n) => Ok(Val::Num(n + 1)),
        _ => Err(Err::TypeError(format!("add1 expected number, got {v}"))),
    }
}

fn eval_sub1(v: Val) -> Result<Val, Err> {
    match v {
        Val::Num(n) => Ok(Val::Num(n - 1)),
        _ => Err(Err::TypeError(format!("sub1 expected number, got {v}"))),
    }
}

fn eval_bin(op: &BinOp, v1: Val, v2: Val) -> Result<Val, Err> {
    match (op, v1, v2) {
        (BinOp::Equal, v1, v2) => Ok(Val::Bool(v1 == v2)),
        (BinOp::Plus, Val::Num(lhs), Val::Num(rhs)) => Ok(Val::Num(lhs + rhs)),
        (BinOp::Minus, Val::Num(lhs), Val::Num(rhs)) => Ok(Val::Num(lhs - rhs)),
        (BinOp::Times, Val::Num(lhs), Val::Num(rhs)) => Ok(Val::Num(lhs * rhs)),
        (BinOp::Greater, Val::Num(lhs), Val::Num(rhs)) => Ok(Val::Bool(lhs > rhs)),
        (BinOp::GreaterEqual, Val::Num(lhs), Val::Num(rhs)) => Ok(Val::Bool(lhs >= rhs)),
        (BinOp::Less, Val::Num(lhs), Val::Num(rhs)) => Ok(Val::Bool(lhs < rhs)),
        (BinOp::LessEqual, Val::Num(lhs), Val::Num(rhs)) => Ok(Val::Bool(lhs <= rhs)),
        (_, v1, v2) => Err(Err::TypeError(format!(
            "operator expected numbers, got {v1} and {v2}"
        ))),
    }
}

pub fn input_var() -> String {
    "#input".to_string()
}

pub fn eval(e: &Expr, env: &mut Env, funs: &FunEnv) -> Result<Val, Err> {
    match e {
        Expr::Number(n) => Ok(Val::Num(*n)),
        Expr::Boolean(b) => Ok(Val::Bool(*b)),
        Expr::Id(x) => eval_id(x, env),
        Expr::Input => Ok(env.get(&input_var()).cloned().unwrap_or(Val::Num(0))),
        Expr::Let(items, expr) => {
            let mut new_env = env.clone();
            for (x, e_i) in items {
                let v = eval(e_i, &mut new_env, funs)?;
                new_env = new_env.update(x.to_string(), v);
            }
            eval(expr, &mut new_env, funs)
        }
        Expr::UnOp(op1, expr) => eval_unop(op1, expr, env, funs),
        Expr::BinOp(op2, expr, expr1) => {
            let v1 = eval(expr, env, funs)?;
            let v2 = eval(expr1, env, funs)?;
            eval_bin(op2, v1, v2)
        }
        Expr::Set(x, expr) => {
            if !env.contains_key(x) {
                return Err(Err::UnboundVar(x.clone()));
            }
            let v = eval(expr, env, funs)?;
            *env = env.update(x.to_string(), v.clone());
            Ok(v)
        }
        Expr::Block(exprs) => {
            let mut val = Val::Num(0);
            for e in exprs {
                val = eval(e, env, funs)?;
            }
            Ok(val)
        }
        Expr::If(cond, then_expr, else_expr) => match eval(cond, env, funs)? {
            Val::Bool(true) => eval(then_expr, env, funs),
            Val::Bool(false) => eval(else_expr, env, funs),
            v => Err(Err::TypeError(format!("if expected bool, got {v}"))),
        },
        Expr::Loop(_) => Err(Err::Unsupported(
            "loop is not supported in this REPL increment".to_string(),
        )),
        Expr::Break(expr) => Err(Err::Break(eval(expr, env, funs)?)),
        Expr::Fn(name, exprs) => eval_call(name, exprs, env, funs),
        Expr::Nil => Ok(Val::Vector(Rc::new(RefCell::new(vec![])))),
        Expr::Vec(exprs) => {
            let mut vals = Vec::with_capacity(exprs.len());
            for expr in exprs {
                vals.push(eval(expr, env, funs)?);
            }
            Ok(Val::Vector(Rc::new(RefCell::new(vals))))
        }
        Expr::VecGet(expr, index) => {
            let vec = expect_vec(eval(expr, env, funs)?, "vec-get")?;
            let vals = vec.borrow();
            let pos = resolve_index(index, vals.len())?;
            Ok(vals[pos].clone())
        }
        Expr::VecLen(expr) => {
            let vec = expect_vec(eval(expr, env, funs)?, "vec-len")?;
            let len = vec.borrow().len() as i64;
            Ok(Val::Num(len))
        }
        Expr::VecSet(expr, index, expr1) => {
            let vec = expect_vec(eval(expr, env, funs)?, "vec-set")?;
            let val = eval(expr1, env, funs)?;
            let mut vals = vec.borrow_mut();
            let pos = resolve_index(index, vals.len())?;
            vals[pos] = val;
            drop(vals);
            Ok(Val::Vector(vec))
        }
    }
}

fn eval_unop(op: &UnOp, expr: &Expr, env: &mut Env, funs: &FunEnv) -> Result<Val, Err> {
    match op {
        UnOp::Add1 => eval_add1(eval(expr, env, funs)?),
        UnOp::Sub1 => eval_sub1(eval(expr, env, funs)?),
        UnOp::IsNum => Ok(Val::Bool(matches!(eval(expr, env, funs)?, Val::Num(_)))),
        UnOp::IsBool => Ok(Val::Bool(matches!(eval(expr, env, funs)?, Val::Bool(_)))),
        UnOp::Print => {
            let v = eval(expr, env, funs)?;
            println!("{v}");
            Ok(v)
        }
    }
}

fn eval_call(name: &str, exprs: &[Expr], env: &mut Env, funs: &FunEnv) -> Result<Val, Err> {
    let defn = funs
        .get(name)
        .ok_or_else(|| Err::UnboundVar(name.to_string()))?
        .clone();

    if defn.params.len() != exprs.len() {
        return Err(Err::TypeError(format!(
            "{name} expected {} arguments, got {}",
            defn.params.len(),
            exprs.len()
        )));
    }

    let mut call_env = HashMap::new();
    for (param, arg) in defn.params.iter().zip(exprs.iter()) {
        let val = eval(arg, env, funs)?;
        call_env = call_env.update(param.clone(), val);
    }

    eval(&defn.body, &mut call_env, funs)
}

fn expect_vec(v: Val, context: &str) -> Result<Rc<RefCell<Vec<Val>>>, Err> {
    match v {
        Val::Vector(vals) => Ok(vals),
        _ => Err(Err::TypeError(format!(
            "{context} expected vector, got {v}"
        ))),
    }
}

fn resolve_index(index: &Index, len: usize) -> Result<usize, Err> {
    if len == 0 {
        return Err(Err::TypeError("index out of bounds".to_string()));
    }

    let pos = match index {
        Index::First => 0,
        Index::Last => len as i64 - 1,
        Index::I(i) if *i < 0 => len as i64 + i,
        Index::I(i) => *i,
    };

    if pos < 0 || pos >= len as i64 {
        return Err(Err::TypeError("index out of bounds".to_string()));
    }

    Ok(pos as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eval_to_string(input: &str) -> String {
        eval_program(input).unwrap().to_string()
    }

    #[test]
    fn counts_parentheses() {
        assert_eq!(paren_delta("(vec 1 2"), 1);
        assert_eq!(paren_delta("3))"), -2);
        assert_eq!(paren_delta("(+ 1 2)"), 0);
    }

    #[test]
    fn evaluates_vec_get() {
        assert_eq!(eval_to_string("(vec-get (vec 1 2 3) 1)"), "2");
    }

    #[test]
    fn evaluates_if() {
        assert_eq!(eval_to_string("(if (= true false) 0 1)"), "1");
    }

    #[test]
    fn prints_nested_vectors() {
        assert_eq!(eval_to_string("(vec 1 (vec 2 3))"), "(vec 1 (vec 2 3))");
    }

    #[test]
    fn vec_set_mutates_shared_vector() {
        let input = "(let ((v (vec 1 2 3))) (block (vec-set v 1 9) v))";
        assert_eq!(eval_to_string(input), "(vec 1 9 3)");
    }
}
