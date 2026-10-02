use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Lt,
    Le,
    Gt,
    Ge,
    Eq,
    Ne,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expr {
    Num(f64),
    /// Reference to a parameter (name as written, later resolved to a key).
    Ref(String),
    Neg(Box<Expr>),
    Not(Box<Expr>),
    Bin(BinOp, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
}

#[derive(Debug, Clone, PartialEq, thiserror::Error)]
#[error("{message} at position {pos}")]
pub struct ParseError {
    pub message: String,
    pub pos: usize,
}

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Ident(String),
    Op(&'static str),
    LParen,
    RParen,
    Comma,
}

fn tokenize(src: &str) -> Result<Vec<(Tok, usize)>, ParseError> {
    let b = src.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < b.len() {
        let c = b[i] as char;
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        if c.is_ascii_digit() || (c == '.' && i + 1 < b.len() && (b[i + 1] as char).is_ascii_digit()) {
            while i < b.len() && ((b[i] as char).is_ascii_digit() || b[i] == b'.') {
                i += 1;
            }
            if i < b.len() && (b[i] == b'e' || b[i] == b'E') {
                let save = i;
                i += 1;
                if i < b.len() && (b[i] == b'+' || b[i] == b'-') {
                    i += 1;
                }
                if i < b.len() && (b[i] as char).is_ascii_digit() {
                    while i < b.len() && (b[i] as char).is_ascii_digit() {
                        i += 1;
                    }
                } else {
                    i = save;
                }
            }
            let text = &src[start..i];
            let v: f64 = text.parse().map_err(|_| ParseError { message: format!("invalid number '{text}'"), pos: start })?;
            out.push((Tok::Num(v), start));
            // Optional unit suffix "mm" is accepted and ignored.
            if src[i..].starts_with("mm") && !src[i + 2..].starts_with(|ch: char| ch.is_alphanumeric() || ch == '_') {
                i += 2;
            }
            continue;
        }
        if c.is_alphabetic() || c == '_' || c == '#' {
            i += 1;
            while i < b.len() {
                let ch = b[i] as char;
                if ch.is_alphanumeric() || ch == '_' || ch == '.' || ch == '#' {
                    i += 1;
                } else {
                    break;
                }
            }
            out.push((Tok::Ident(src[start..i].trim_end_matches('.').to_string()), start));
            continue;
        }
        let two = if i + 1 < b.len() { &src[i..i + 2] } else { "" };
        let op2 = match two {
            "<=" => Some("<="),
            ">=" => Some(">="),
            "==" => Some("=="),
            "!=" => Some("!="),
            "&&" => Some("&&"),
            "||" => Some("||"),
            _ => None,
        };
        if let Some(op) = op2 {
            out.push((Tok::Op(op), start));
            i += 2;
            continue;
        }
        let tok = match c {
            '+' => Tok::Op("+"),
            '-' => Tok::Op("-"),
            '*' => Tok::Op("*"),
            '/' => Tok::Op("/"),
            '<' => Tok::Op("<"),
            '>' => Tok::Op(">"),
            '!' => Tok::Op("!"),
            '(' => Tok::LParen,
            ')' => Tok::RParen,
            ',' => Tok::Comma,
            _ => return Err(ParseError { message: format!("unexpected character '{c}'"), pos: start }),
        };
        out.push((tok, start));
        i += 1;
    }
    Ok(out)
}

struct Parser {
    toks: Vec<(Tok, usize)>,
    i: usize,
    len: usize,
}

const FUNCTIONS: &[(&str, usize, usize)] = &[
    ("min", 1, usize::MAX),
    ("max", 1, usize::MAX),
    ("clamp", 3, 3),
    ("if", 3, 3),
    ("abs", 1, 1),
    ("round", 1, 1),
    ("floor", 1, 1),
    ("ceil", 1, 1),
    ("sqrt", 1, 1),
];

impl Parser {
    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.i).map(|t| &t.0)
    }

    fn pos(&self) -> usize {
        self.toks.get(self.i).map(|t| t.1).unwrap_or(self.len)
    }

    fn err<T>(&self, m: impl Into<String>) -> Result<T, ParseError> {
        Err(ParseError { message: m.into(), pos: self.pos() })
    }

    fn bin_prec(op: &str) -> Option<(u8, BinOp)> {
        Some(match op {
            "||" => (1, BinOp::Or),
            "&&" => (2, BinOp::And),
            "==" => (3, BinOp::Eq),
            "!=" => (3, BinOp::Ne),
            "<" => (4, BinOp::Lt),
            "<=" => (4, BinOp::Le),
            ">" => (4, BinOp::Gt),
            ">=" => (4, BinOp::Ge),
            "+" => (5, BinOp::Add),
            "-" => (5, BinOp::Sub),
            "*" => (6, BinOp::Mul),
            "/" => (6, BinOp::Div),
            _ => return None,
        })
    }

    fn expr(&mut self, min_prec: u8) -> Result<Expr, ParseError> {
        let mut lhs = self.unary()?;
        while let Some(Tok::Op(op)) = self.peek() {
            let Some((prec, bop)) = Self::bin_prec(op) else { break };
            if prec < min_prec {
                break;
            }
            self.i += 1;
            let rhs = self.expr(prec + 1)?;
            lhs = Expr::Bin(bop, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> Result<Expr, ParseError> {
        match self.peek() {
            Some(Tok::Op("-")) => {
                self.i += 1;
                Ok(Expr::Neg(Box::new(self.unary()?)))
            }
            Some(Tok::Op("+")) => {
                self.i += 1;
                self.unary()
            }
            Some(Tok::Op("!")) => {
                self.i += 1;
                Ok(Expr::Not(Box::new(self.unary()?)))
            }
            _ => self.primary(),
        }
    }

    fn primary(&mut self) -> Result<Expr, ParseError> {
        match self.peek().cloned() {
            Some(Tok::Num(v)) => {
                self.i += 1;
                Ok(Expr::Num(v))
            }
            Some(Tok::LParen) => {
                self.i += 1;
                let e = self.expr(0)?;
                if self.peek() != Some(&Tok::RParen) {
                    return self.err("expected ')'");
                }
                self.i += 1;
                Ok(e)
            }
            Some(Tok::Ident(name)) => {
                self.i += 1;
                if self.peek() == Some(&Tok::LParen) {
                    let Some(&(_, lo, hi)) = FUNCTIONS.iter().find(|f| f.0 == name) else {
                        return self.err(format!("unknown function '{name}'"));
                    };
                    self.i += 1;
                    let mut args = Vec::new();
                    if self.peek() != Some(&Tok::RParen) {
                        loop {
                            args.push(self.expr(0)?);
                            match self.peek() {
                                Some(Tok::Comma) => self.i += 1,
                                Some(Tok::RParen) => break,
                                _ => return self.err("expected ',' or ')'"),
                            }
                        }
                    }
                    self.i += 1;
                    if args.len() < lo || args.len() > hi {
                        return self.err(format!("wrong number of arguments for '{name}'"));
                    }
                    Ok(Expr::Call(name, args))
                } else if name == "true" {
                    Ok(Expr::Num(1.0))
                } else if name == "false" {
                    Ok(Expr::Num(0.0))
                } else {
                    Ok(Expr::Ref(name))
                }
            }
            _ => self.err("expected a value"),
        }
    }
}

/// Parse an expression. A leading `=` (spreadsheet style) is allowed.
pub fn parse(src: &str) -> Result<Expr, ParseError> {
    let trimmed = src.trim();
    let offset = src.len() - src.trim_start().len();
    let body = trimmed.strip_prefix('=').unwrap_or(trimmed);
    let shift = offset + (trimmed.len() - body.len());
    let toks = tokenize(body).map_err(|e| ParseError { pos: e.pos + shift, ..e })?;
    if toks.is_empty() {
        return Err(ParseError { message: "empty expression".into(), pos: shift });
    }
    let mut p = Parser { toks, i: 0, len: body.len() };
    let e = p.expr(0).map_err(|e| ParseError { pos: e.pos + shift, ..e })?;
    if p.i != p.toks.len() {
        return Err(ParseError { message: "unexpected trailing input".into(), pos: p.pos() + shift });
    }
    Ok(e)
}

impl Expr {
    pub fn refs(&self, out: &mut Vec<String>) {
        match self {
            Expr::Num(_) => {}
            Expr::Ref(r) => out.push(r.clone()),
            Expr::Neg(e) | Expr::Not(e) => e.refs(out),
            Expr::Bin(_, a, b) => {
                a.refs(out);
                b.refs(out);
            }
            Expr::Call(_, args) => args.iter().for_each(|a| a.refs(out)),
        }
    }

    /// Rewrite every reference through `f`.
    pub fn map_refs(&self, f: &mut impl FnMut(&str) -> Result<String, String>) -> Result<Expr, String> {
        Ok(match self {
            Expr::Num(v) => Expr::Num(*v),
            Expr::Ref(r) => Expr::Ref(f(r)?),
            Expr::Neg(e) => Expr::Neg(Box::new(e.map_refs(f)?)),
            Expr::Not(e) => Expr::Not(Box::new(e.map_refs(f)?)),
            Expr::Bin(op, a, b) => Expr::Bin(*op, Box::new(a.map_refs(f)?), Box::new(b.map_refs(f)?)),
            Expr::Call(n, args) => Expr::Call(n.clone(), args.iter().map(|a| a.map_refs(f)).collect::<Result<_, _>>()?),
        })
    }

    pub fn is_literal(&self) -> bool {
        match self {
            Expr::Num(_) => true,
            Expr::Neg(e) => e.is_literal(),
            _ => false,
        }
    }

    /// Evaluate with a lookup for references.
    pub fn eval(&self, lookup: &impl Fn(&str) -> Result<f64, String>) -> Result<f64, String> {
        let b = |v: bool| if v { 1.0 } else { 0.0 };
        Ok(match self {
            Expr::Num(v) => *v,
            Expr::Ref(r) => lookup(r)?,
            Expr::Neg(e) => -e.eval(lookup)?,
            Expr::Not(e) => b(e.eval(lookup)? == 0.0),
            Expr::Bin(op, x, y) => {
                let a = x.eval(lookup)?;
                // Short-circuit logic.
                match op {
                    BinOp::And if a == 0.0 => return Ok(0.0),
                    BinOp::Or if a != 0.0 => return Ok(1.0),
                    _ => {}
                }
                let c = y.eval(lookup)?;
                match op {
                    BinOp::Add => a + c,
                    BinOp::Sub => a - c,
                    BinOp::Mul => a * c,
                    BinOp::Div => {
                        if c == 0.0 {
                            return Err("division by zero".into());
                        }
                        a / c
                    }
                    BinOp::Lt => b(a < c),
                    BinOp::Le => b(a <= c),
                    BinOp::Gt => b(a > c),
                    BinOp::Ge => b(a >= c),
                    BinOp::Eq => b((a - c).abs() < 1e-9),
                    BinOp::Ne => b((a - c).abs() >= 1e-9),
                    BinOp::And | BinOp::Or => b(c != 0.0),
                }
            }
            Expr::Call(name, args) => match name.as_str() {
                "if" => {
                    if args[0].eval(lookup)? != 0.0 {
                        args[1].eval(lookup)?
                    } else {
                        args[2].eval(lookup)?
                    }
                }
                _ => {
                    let v: Vec<f64> = args.iter().map(|a| a.eval(lookup)).collect::<Result<_, _>>()?;
                    match name.as_str() {
                        "min" => v.iter().copied().fold(f64::INFINITY, f64::min),
                        "max" => v.iter().copied().fold(f64::NEG_INFINITY, f64::max),
                        "clamp" => v[0].max(v[1]).min(v[2]),
                        "abs" => v[0].abs(),
                        "round" => v[0].round(),
                        "floor" => v[0].floor(),
                        "ceil" => v[0].ceil(),
                        "sqrt" => {
                            if v[0] < 0.0 {
                                return Err("sqrt of negative".into());
                            }
                            v[0].sqrt()
                        }
                        _ => return Err(format!("unknown function '{name}'")),
                    }
                }
            },
        })
    }
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expr::Num(v) => write!(f, "{v}"),
            Expr::Ref(r) => f.write_str(r),
            Expr::Neg(e) => write!(f, "-({e})"),
            Expr::Not(e) => write!(f, "!({e})"),
            Expr::Bin(op, a, b) => {
                let s = match op {
                    BinOp::Add => "+",
                    BinOp::Sub => "-",
                    BinOp::Mul => "*",
                    BinOp::Div => "/",
                    BinOp::Lt => "<",
                    BinOp::Le => "<=",
                    BinOp::Gt => ">",
                    BinOp::Ge => ">=",
                    BinOp::Eq => "==",
                    BinOp::Ne => "!=",
                    BinOp::And => "&&",
                    BinOp::Or => "||",
                };
                write!(f, "({a} {s} {b})")
            }
            Expr::Call(n, args) => {
                write!(f, "{n}(")?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        f.write_str(", ")?;
                    }
                    write!(f, "{a}")?;
                }
                f.write_str(")")
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ev(s: &str) -> f64 {
        parse(s).unwrap().eval(&|r: &str| match r {
            "cabinet.width" => Ok(800.0),
            "t" => Ok(18.0),
            _ => Err(format!("unknown {r}")),
        })
        .unwrap()
    }

    #[test]
    fn arithmetic_and_precedence() {
        assert_eq!(ev("1 + 2 * 3"), 7.0);
        assert_eq!(ev("(1 + 2) * 3"), 9.0);
        assert_eq!(ev("-2 * -3"), 6.0);
        assert_eq!(ev("= cabinet.width - 2 * t"), 764.0);
        assert_eq!(ev("800mm / 2"), 400.0);
    }

    #[test]
    fn functions_and_logic() {
        assert_eq!(ev("min(3, 1, 2)"), 1.0);
        assert_eq!(ev("max(3, 1, 2)"), 3.0);
        assert_eq!(ev("clamp(5, 0, 2)"), 2.0);
        assert_eq!(ev("if(cabinet.width > 600, 2, 1)"), 2.0);
        assert_eq!(ev("t >= 3 && t <= 60"), 1.0);
    }

    #[test]
    fn errors() {
        assert!(parse("1 +").is_err());
        assert!(parse("foo(1)").is_err());
        assert!(parse("clamp(1,2)").is_err());
        assert!(parse("1 ; drop").is_err());
        assert!(parse("1/0").unwrap().eval(&|_| Ok(0.0)).is_err());
    }
}
