//! First slice of Frihart's own script engine.
//!
//! This is not JavaScript and not Boa. It evaluates arithmetic so the
//! folder already runs code we wrote. Numbers, parentheses, unary minus,
//! and `+ - * /`. Nothing else. No host objects, no network.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptError {
    Empty,
    BadNumber,
    Unexpected,
    Trailing,
    DivZero,
}

struct Parser<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> Parser<'a> {
    fn skip(&mut self) {
        while self.i < self.s.len() && self.s[self.i].is_ascii_whitespace() {
            self.i += 1;
        }
    }

    fn eof(&self) -> bool {
        self.i >= self.s.len()
    }

    fn peek(&mut self) -> Option<u8> {
        self.skip();
        self.s.get(self.i).copied()
    }

    fn eat(&mut self, c: u8) -> bool {
        if self.peek() == Some(c) {
            self.i += 1;
            true
        } else {
            false
        }
    }

    fn expr(&mut self) -> Result<f64, ScriptError> {
        let mut v = self.term()?;
        loop {
            if self.eat(b'+') {
                v += self.term()?;
            } else if self.eat(b'-') {
                v -= self.term()?;
            } else {
                break;
            }
        }
        Ok(v)
    }

    fn term(&mut self) -> Result<f64, ScriptError> {
        let mut v = self.unary()?;
        loop {
            if self.eat(b'*') {
                v *= self.unary()?;
            } else if self.eat(b'/') {
                let d = self.unary()?;
                if d == 0.0 {
                    return Err(ScriptError::DivZero);
                }
                v /= d;
            } else {
                break;
            }
        }
        Ok(v)
    }

    fn unary(&mut self) -> Result<f64, ScriptError> {
        if self.eat(b'-') {
            Ok(-self.unary()?)
        } else if self.eat(b'+') {
            self.unary()
        } else {
            self.primary()
        }
    }

    fn primary(&mut self) -> Result<f64, ScriptError> {
        if self.eat(b'(') {
            let v = self.expr()?;
            if !self.eat(b')') {
                return Err(ScriptError::Unexpected);
            }
            return Ok(v);
        }
        self.number()
    }

    fn number(&mut self) -> Result<f64, ScriptError> {
        self.skip();
        let start = self.i;
        while self.i < self.s.len() && self.s[self.i].is_ascii_digit() {
            self.i += 1;
        }
        if self.i < self.s.len() && self.s[self.i] == b'.' {
            self.i += 1;
            let frac = self.i;
            while self.i < self.s.len() && self.s[self.i].is_ascii_digit() {
                self.i += 1;
            }
            if self.i == frac {
                return Err(ScriptError::BadNumber);
            }
        }
        if self.i == start {
            return Err(ScriptError::Unexpected);
        }
        let text =
            std::str::from_utf8(&self.s[start..self.i]).map_err(|_| ScriptError::BadNumber)?;
        text.parse().map_err(|_| ScriptError::BadNumber)
    }
}

pub fn eval(src: &str) -> Result<f64, ScriptError> {
    let mut p = Parser {
        s: src.as_bytes(),
        i: 0,
    };
    p.skip();
    if p.eof() {
        return Err(ScriptError::Empty);
    }
    let v = p.expr()?;
    p.skip();
    if !p.eof() {
        return Err(ScriptError::Trailing);
    }
    Ok(v)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_and_precedence() {
        assert_eq!(eval("1+2*3").unwrap(), 7.0);
        assert_eq!(eval("(1+2)*3").unwrap(), 9.0);
        assert_eq!(eval("-4+1").unwrap(), -3.0);
        assert_eq!(eval("10/4").unwrap(), 2.5);
    }

    #[test]
    fn refuses_junk_and_division_by_zero() {
        assert_eq!(eval("").unwrap_err(), ScriptError::Empty);
        assert_eq!(eval("1+").unwrap_err(), ScriptError::Unexpected);
        assert_eq!(eval("1 2").unwrap_err(), ScriptError::Trailing);
        assert_eq!(eval("1/0").unwrap_err(), ScriptError::DivZero);
        assert!(eval("fetch(1)").is_err());
    }
}
