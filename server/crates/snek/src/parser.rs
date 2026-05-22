use crate::expr::*;
use sexp::Atom::*;
use sexp::*;
use std::collections::HashSet;
use std::fmt;

const KEYWORDS: [&str; 22] = [
    "add1", "sub1", "isnum", "isbool", "+", "-", "*", "<", ">", ">=", "<=", "=", "input", "let",
    "set!", "if", "block", "loop", "break", "true", "false", "print",
];

const MIN_NUM: i64 = -(1_i64 << 62);
const MAX_NUM: i64 = (1_i64 << 62) - 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub message: String,
    pub line: Option<usize>,
    pub column: Option<usize>,
    pub index: Option<usize>,
}

impl ParseError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            line: None,
            column: None,
            index: None,
        }
    }

    fn from_sexp_error(err: &sexp::Error) -> Self {
        Self {
            message: err.message.to_string(),
            line: Some(err.line),
            column: Some(err.column),
            index: Some(err.index),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.line, self.column) {
            (Some(line), Some(column)) => write!(f, "{line}:{column}: {}", self.message),
            _ => write!(f, "{}", self.message),
        }
    }
}

fn valid_id(s: &str) -> bool {
    !KEYWORDS.contains(&s)
}

fn reserved_form(s: &str) -> bool {
    KEYWORDS.contains(&s) || matches!(s, "fun" | "nil" | "vec" | "vec-get" | "vec-len" | "vec-set")
}

fn error<T>(message: impl Into<String>) -> Result<T, Vec<ParseError>> {
    Err(vec![ParseError::new(message)])
}

fn format_parse_errors(errors: &[ParseError]) -> String {
    errors
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn parse(s: &str) -> Prog {
    parse_program(s).unwrap_or_else(|errors| panic!("{}", format_parse_errors(&errors)))
}

pub fn parse_program(source: &str) -> Result<Prog, Vec<ParseError>> {
    let source = format!("({source})");
    let sexp = sexp::parse(&source).map_err(|err| vec![ParseError::from_sexp_error(&err)])?;
    parse_prog(&sexp)
}

pub fn parse_expr(source: &str) -> Result<Expr, Vec<ParseError>> {
    let sexp = sexp::parse(source).map_err(|err| vec![ParseError::from_sexp_error(&err)])?;
    parse_expr_sexp(&sexp)
}

fn parse_prog(s: &Sexp) -> Result<Prog, Vec<ParseError>> {
    let Sexp::List(es) = s else {
        return error("syntax error: program must be a list");
    };

    let [defs @ .., expr] = &es[..] else {
        return error("syntax error: program must contain a main expression");
    };

    let mut errors = Vec::new();
    let mut defns = Vec::new();

    for defn in defs {
        match parse_defn(defn) {
            Ok(defn) => defns.push(defn),
            Err(mut defn_errors) => errors.append(&mut defn_errors),
        }
    }

    let mut fn_names: HashSet<String> = HashSet::new();
    for d in &defns {
        if !fn_names.insert(d.name.clone()) {
            errors.push(ParseError::new(format!("duplicate function name `{}`", d.name)));
        }
    }

    let main = match parse_expr_sexp(expr) {
        Ok(expr) => Some(expr),
        Err(mut expr_errors) => {
            errors.append(&mut expr_errors);
            None
        }
    };

    if errors.is_empty() {
        Ok(Prog {
            defns,
            main: main.expect("main expression parsed when no parser errors exist"),
        })
    } else {
        Err(errors)
    }
}

fn parse_defn(s: &Sexp) -> Result<Defn, Vec<ParseError>> {
    let Sexp::List(es) = s else {
        return error("syntax error: expected a function definition list");
    };

    match &es[..] {
        [Sexp::Atom(S(op)), Sexp::List(es), body] if op == "fun" => {
            let [name, params @ ..] = &es[..] else {
                return error("missing function name");
            };

            let mut errors = Vec::new();

            let name = match parse_ident(name) {
                Ok(name) => Some(name),
                Err(mut ident_errors) => {
                    errors.append(&mut ident_errors);
                    None
                }
            };

            let params = match parse_params(params) {
                Ok(params) => Some(params),
                Err(mut param_errors) => {
                    errors.append(&mut param_errors);
                    None
                }
            };

            let body = match parse_expr_sexp(body) {
                Ok(body) => Some(body),
                Err(mut body_errors) => {
                    errors.append(&mut body_errors);
                    None
                }
            };

            if errors.is_empty() {
                Ok(Defn {
                    name: name.expect("function name parsed when no parser errors exist"),
                    params: params.expect("params parsed when no parser errors exist"),
                    body: body.expect("body parsed when no parser errors exist"),
                })
            } else {
                Err(errors)
            }
        }
        _ => error("syntax error: expected `(fun (name params...) body)`"),
    }
}

fn parse_ident(s: &Sexp) -> Result<String, Vec<ParseError>> {
    match s {
        Sexp::Atom(S(x)) if valid_id(x) => Ok(x.to_string()),
        Sexp::Atom(S(x)) => error(format!("invalid keyword identifier `{x}`")),
        _ => error("parse error: expected identifier"),
    }
}

fn parse_params(params: &[Sexp]) -> Result<Vec<String>, Vec<ParseError>> {
    let mut seen = HashSet::new();
    let mut errors = Vec::new();
    let mut names = Vec::new();

    for param in params {
        match parse_ident(param) {
            Ok(name) => {
                if !seen.insert(name.clone()) {
                    errors.push(ParseError::new(format!("duplicate parameter name `{name}`")));
                }
                names.push(name);
            }
            Err(mut param_errors) => errors.append(&mut param_errors),
        }
    }

    if errors.is_empty() {
        Ok(names)
    } else {
        Err(errors)
    }
}

fn parse_bind(s: &Sexp) -> Result<(String, Expr), Vec<ParseError>> {
    match s {
        Sexp::List(sexps) => match &sexps[..] {
            [Sexp::Atom(S(x)), e] if valid_id(x) => {
                Ok((x.to_string(), parse_expr_sexp(e)?))
            }
            [Sexp::Atom(S(x)), _] => error(format!("invalid keyword binding `{x}`")),
            _ => error("invalid binding: expected `(name expr)`"),
        },
        _ => error("invalid binding: expected `(name expr)`"),
    }
}

fn parse_bindings(s: &Sexp) -> Result<Vec<(String, Expr)>, Vec<ParseError>> {
    match s {
        Sexp::List(bindings) if bindings.is_empty() => error("invalid let: expected at least one binding"),
        Sexp::List(bindings) => {
            let mut errors = Vec::new();
            let mut parsed_bindings = Vec::new();

            for binding in bindings {
                match parse_bind(binding) {
                    Ok(binding) => parsed_bindings.push(binding),
                    Err(mut binding_errors) => errors.append(&mut binding_errors),
                }
            }

            if errors.is_empty() {
                Ok(parsed_bindings)
            } else {
                Err(errors)
            }
        }
        _ => error("invalid let: expected binding list"),
    }
}

fn parse_index(s: &Sexp) -> Result<Index, Vec<ParseError>> {
    match s {
        Sexp::Atom(I(0)) => Ok(Index::First),
        Sexp::Atom(I(-1)) => Ok(Index::Last),
        Sexp::Atom(I(i)) => Ok(Index::I(*i)),
        Sexp::Atom(S(s)) if s == "first" => Ok(Index::First),
        Sexp::Atom(S(s)) if s == "last" => Ok(Index::Last),
        _ => error("syntax error: index must be `first`, `last`, or an integer"),
    }
}

fn parse_exprs<'a, I>(exprs: I) -> Result<Vec<Expr>, Vec<ParseError>>
where
    I: IntoIterator<Item = &'a Sexp>,
{
    let mut errors = Vec::new();
    let mut parsed_exprs = Vec::new();

    for expr in exprs {
        match parse_expr_sexp(expr) {
            Ok(expr) => parsed_exprs.push(expr),
            Err(mut expr_errors) => errors.append(&mut expr_errors),
        }
    }

    if errors.is_empty() {
        Ok(parsed_exprs)
    } else {
        Err(errors)
    }
}

fn parse_expr_sexp(s: &Sexp) -> Result<Expr, Vec<ParseError>> {
    match s {
        Sexp::Atom(I(n)) => {
            if *n < MIN_NUM || *n > MAX_NUM {
                error("overflow")
            } else {
                Ok(num(*n))
            }
        }
        Sexp::Atom(S(s)) if s == "true" => Ok(Expr::Boolean(true)),
        Sexp::Atom(S(s)) if s == "false" => Ok(Expr::Boolean(false)),
        Sexp::Atom(S(s)) if s == "input" => Ok(Expr::Input),
        Sexp::Atom(S(s)) if s == "nil" => Ok(Expr::Nil),
        Sexp::Atom(S(s)) if valid_id(s) => Ok(Expr::Id(s.clone())),
        Sexp::Atom(S(s)) => error(format!("invalid keyword `{s}`")),
        Sexp::List(sexps) => match &sexps[..] {
            // op1
            [Sexp::Atom(S(op)), e] if op == "add1" => Ok(add1(parse_expr_sexp(e)?)),
            [Sexp::Atom(S(op)), e] if op == "sub1" => Ok(sub1(parse_expr_sexp(e)?)),
            [Sexp::Atom(S(op)), e] if op == "isnum" => Ok(isnum(parse_expr_sexp(e)?)),
            [Sexp::Atom(S(op)), e] if op == "isbool" => Ok(isbool(parse_expr_sexp(e)?)),
            [Sexp::Atom(S(op)), e] if op == "print" => Ok(print(parse_expr_sexp(e)?)),
            [Sexp::Atom(S(op)), e] if op == "vec-len" => {
                Ok(Expr::VecLen(Box::new(parse_expr_sexp(e)?)))
            }

            // op2
            [Sexp::Atom(S(op)), e1, e2] if op == "+" => {
                Ok(plus(parse_expr_sexp(e1)?, parse_expr_sexp(e2)?))
            }
            [Sexp::Atom(S(op)), e1, e2] if op == "-" => {
                Ok(minus(parse_expr_sexp(e1)?, parse_expr_sexp(e2)?))
            }
            [Sexp::Atom(S(op)), e1, e2] if op == "*" => {
                Ok(times(parse_expr_sexp(e1)?, parse_expr_sexp(e2)?))
            }
            [Sexp::Atom(S(op)), e1, e2] if op == "<" => {
                Ok(lt(parse_expr_sexp(e1)?, parse_expr_sexp(e2)?))
            }
            [Sexp::Atom(S(op)), e1, e2] if op == ">" => {
                Ok(gt(parse_expr_sexp(e1)?, parse_expr_sexp(e2)?))
            }
            [Sexp::Atom(S(op)), e1, e2] if op == "<=" => {
                Ok(le(parse_expr_sexp(e1)?, parse_expr_sexp(e2)?))
            }
            [Sexp::Atom(S(op)), e1, e2] if op == ">=" => {
                Ok(ge(parse_expr_sexp(e1)?, parse_expr_sexp(e2)?))
            }
            [Sexp::Atom(S(op)), e1, e2] if op == "=" => {
                Ok(eq(parse_expr_sexp(e1)?, parse_expr_sexp(e2)?))
            }
            [Sexp::Atom(S(op)), e1, e2] if op == "vec-get" => {
                Ok(Expr::VecGet(Box::new(parse_expr_sexp(e1)?), parse_index(e2)?))
            }

            // op3
            [Sexp::Atom(S(op)), e1, e2, e3] if op == "vec-set" => Ok(Expr::VecSet(
                Box::new(parse_expr_sexp(e1)?),
                parse_index(e2)?,
                Box::new(parse_expr_sexp(e3)?),
            )),

            // scoped
            [Sexp::Atom(S(op)), binds, e] if op == "let" => {
                Ok(Expr::Let(parse_bindings(binds)?, Box::new(parse_expr_sexp(e)?)))
            }
            [Sexp::Atom(S(op)), Sexp::Atom(S(bind)), e] if op == "set!" && valid_id(bind) => {
                Ok(Expr::Set(bind.to_string(), Box::new(parse_expr_sexp(e)?)))
            }
            [Sexp::Atom(S(op)), Sexp::Atom(S(bind)), _] if op == "set!" => {
                error(format!("invalid keyword binding `{bind}`"))
            }
            [Sexp::Atom(S(op)), e1, e2, e3] if op == "if" => Ok(Expr::If(
                Box::new(parse_expr_sexp(e1)?),
                Box::new(parse_expr_sexp(e2)?),
                Box::new(parse_expr_sexp(e3)?),
            )),
            [Sexp::Atom(S(op)), e] if op == "loop" => Ok(Expr::Loop(Box::new(parse_expr_sexp(e)?))),
            [Sexp::Atom(S(op)), e] if op == "break" => {
                Ok(Expr::Break(Box::new(parse_expr_sexp(e)?)))
            }
            [Sexp::Atom(S(op)), exprs @ ..] if op == "block" => {
                if exprs.is_empty() {
                    error("invalid block: expected at least one expression")
                } else {
                    Ok(Expr::Block(parse_exprs(exprs.iter())?))
                }
            }
            [Sexp::Atom(S(op)), exprs @ ..] if op == "vec" => {
                Ok(Expr::Vec(parse_exprs(exprs.iter())?))
            }
            [Sexp::Atom(S(f)), exprs @ ..] if !reserved_form(f) => {
                Ok(Expr::Fn(f.to_string(), parse_exprs(exprs.iter())?))
            }
            [Sexp::Atom(S(op)), ..] => error(format!("invalid form for `{op}`")),
            _ => error("invalid expression"),
        },
        _ => error("invalid expression"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_valid_expression_source() {
        assert_eq!(parse_expr("(+ 1 2)").unwrap(), plus(num(1), num(2)));
    }

    #[test]
    fn malformed_sexp_returns_location() {
        let errors = parse_expr("(+ 1").unwrap_err();
        assert_eq!(errors[0].line, Some(1));
        assert!(errors[0].column.is_some());
        assert!(errors[0].index.is_some());
    }

    #[test]
    fn invalid_keyword_identifier_errors() {
        let errors = parse_expr("add1").unwrap_err();
        assert!(errors[0].message.contains("invalid keyword"));
    }

    #[test]
    fn empty_block_errors() {
        let errors = parse_expr("(block)").unwrap_err();
        assert!(errors[0].message.contains("block"));
    }

    #[test]
    fn duplicate_function_names_error() {
        let errors = parse_program("(fun (f) 1) (fun (f) 2) 3").unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.message.contains("duplicate function name")));
    }

    #[test]
    fn duplicate_parameter_names_error() {
        let errors = parse_program("(fun (f x x) x) (f 1)").unwrap_err();
        assert!(errors
            .iter()
            .any(|error| error.message.contains("duplicate parameter name")));
    }

    #[test]
    fn numeric_overflow_errors() {
        let errors = parse_expr("4611686018427387904").unwrap_err();
        assert!(errors[0].message.contains("overflow"));
    }

    #[test]
    fn compatibility_parse_still_returns_program() {
        assert_eq!(parse("1").main, num(1));
    }
}
