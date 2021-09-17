use std::iter::FromIterator;

use super::super::ast::*;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;

#[derive(Debug, FromPrimitive, PartialEq, Eq)]
pub enum Nonterminal {
  Expr,
  Stmt,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Symbol {
  Nonterm(Nonterminal),
  Term(TokenKind),
}

#[derive(Debug, Clone, Default)]
pub struct SymbolSet([u64; 3]);

impl SymbolSet {
  pub fn new() -> Self {
    Self([0; 3])
  }

  pub fn iter(&self) -> SymbolIter {
    self.into_iter()
  }

  pub fn contains_token(&self, tok: TokenKind) -> bool {
    let i: usize = tok.into();
    (self.0[i >> 6] & (1 << (i & 63))) != 0
  }
}

impl Extend<Symbol> for SymbolSet {
  fn extend<I: IntoIterator<Item = Symbol>>(&mut self, iter: I) {
    for sym in iter {
      let k: usize = sym.into();
      self.0[k >> 6] |= 1 << (k & 63);
    }
  }
}

impl FromIterator<Symbol> for SymbolSet {
  fn from_iter<I: IntoIterator<Item = Symbol>>(iter: I) -> Self {
    let mut set = Self::new();
    for sym in iter {
      set.extend_one(sym);
    }
    set
  }
}

impl From<Symbol> for usize {
  fn from(sym: Symbol) -> Self {
    match sym {
      Symbol::Term(t) => t.into(),
      Symbol::Nonterm(n) => 128 + n as usize,
    }
  }
}

impl From<usize> for Symbol {
  fn from(k: usize) -> Self {
    if k < 160 {
      Self::Term(TokenKind::from(k))
    } else {
      Self::Nonterm(Nonterminal::from_usize(k - 160).unwrap())
    }
  }
}

impl From<TokenKind> for Symbol {
  fn from(t: TokenKind) -> Self {
    Self::Term(t)
  }
}

impl From<Nonterminal> for Symbol {
  fn from(n: Nonterminal) -> Self {
    Self::Nonterm(n)
  }
}

impl Nonterminal {
  pub fn first_symbols(&self) -> impl IntoIterator<Item = Symbol> {
    match self {
      Self::Expr => {
        vec![
          Symbol::Term(TokenKind::Float),
          Symbol::Term(TokenKind::Label),
          Symbol::Term(TokenKind::String),
          Symbol::Term(TokenKind::Keyword(Keyword::Inkey)),
          Symbol::Term(TokenKind::Punc(Punc::Plus)),
          Symbol::Term(TokenKind::Punc(Punc::Minus)),
          Symbol::Term(TokenKind::Keyword(Keyword::Not)),
          Symbol::Term(TokenKind::Punc(Punc::LParen)),
          Symbol::Term(TokenKind::Keyword(Keyword::Fn)),
          // any SysFuncKind will work
          Symbol::Term(TokenKind::SysFunc(SysFuncKind::Abs)),
          Symbol::Term(TokenKind::Ident),
        ]
      }
      Self::Stmt => {
        vec![
          Symbol::Term(TokenKind::Keyword(Keyword::Auto)),
          Symbol::Term(TokenKind::Keyword(Keyword::Beep)),
          Symbol::Term(TokenKind::Keyword(Keyword::Box)),
          Symbol::Term(TokenKind::Keyword(Keyword::Call)),
          Symbol::Term(TokenKind::Keyword(Keyword::Circle)),
          Symbol::Term(TokenKind::Keyword(Keyword::Clear)),
          Symbol::Term(TokenKind::Keyword(Keyword::Close)),
          Symbol::Term(TokenKind::Keyword(Keyword::Cls)),
          Symbol::Term(TokenKind::Keyword(Keyword::Cont)),
          Symbol::Term(TokenKind::Keyword(Keyword::Copy)),
          Symbol::Term(TokenKind::Keyword(Keyword::Data)),
          Symbol::Term(TokenKind::Keyword(Keyword::Def)),
          Symbol::Term(TokenKind::Keyword(Keyword::Del)),
          Symbol::Term(TokenKind::Keyword(Keyword::Dim)),
          Symbol::Term(TokenKind::Keyword(Keyword::Draw)),
          Symbol::Term(TokenKind::Keyword(Keyword::Edit)),
          Symbol::Term(TokenKind::Keyword(Keyword::Ellipse)),
          Symbol::Term(TokenKind::Keyword(Keyword::End)),
          Symbol::Term(TokenKind::Keyword(Keyword::Field)),
          Symbol::Term(TokenKind::Keyword(Keyword::Files)),
          Symbol::Term(TokenKind::Keyword(Keyword::Flash)),
          Symbol::Term(TokenKind::Keyword(Keyword::For)),
          Symbol::Term(TokenKind::Keyword(Keyword::Get)),
          Symbol::Term(TokenKind::Keyword(Keyword::Gosub)),
          Symbol::Term(TokenKind::Keyword(Keyword::Goto)),
          Symbol::Term(TokenKind::Keyword(Keyword::Graph)),
          Symbol::Term(TokenKind::Keyword(Keyword::If)),
          Symbol::Term(TokenKind::Keyword(Keyword::Inkey)),
          Symbol::Term(TokenKind::Keyword(Keyword::Input)),
          Symbol::Term(TokenKind::Keyword(Keyword::Inverse)),
          Symbol::Term(TokenKind::Keyword(Keyword::Kill)),
          Symbol::Term(TokenKind::Keyword(Keyword::Let)),
          Symbol::Term(TokenKind::Keyword(Keyword::Line)),
          Symbol::Term(TokenKind::Keyword(Keyword::List)),
          Symbol::Term(TokenKind::Keyword(Keyword::Load)),
          Symbol::Term(TokenKind::Keyword(Keyword::Locate)),
          Symbol::Term(TokenKind::Keyword(Keyword::Lset)),
          Symbol::Term(TokenKind::Keyword(Keyword::New)),
          Symbol::Term(TokenKind::Keyword(Keyword::Next)),
          Symbol::Term(TokenKind::Keyword(Keyword::Normal)),
          Symbol::Term(TokenKind::Keyword(Keyword::Notrace)),
          Symbol::Term(TokenKind::Keyword(Keyword::On)),
          Symbol::Term(TokenKind::Keyword(Keyword::Open)),
          Symbol::Term(TokenKind::Keyword(Keyword::Play)),
          Symbol::Term(TokenKind::Keyword(Keyword::Poke)),
          Symbol::Term(TokenKind::Keyword(Keyword::Pop)),
          Symbol::Term(TokenKind::Keyword(Keyword::Print)),
          Symbol::Term(TokenKind::Keyword(Keyword::Put)),
          Symbol::Term(TokenKind::Keyword(Keyword::Read)),
          Symbol::Term(TokenKind::Keyword(Keyword::Rem)),
          Symbol::Term(TokenKind::Keyword(Keyword::Rename)),
          Symbol::Term(TokenKind::Keyword(Keyword::Restore)),
          Symbol::Term(TokenKind::Keyword(Keyword::Return)),
          Symbol::Term(TokenKind::Keyword(Keyword::Rset)),
          Symbol::Term(TokenKind::Keyword(Keyword::Run)),
          Symbol::Term(TokenKind::Keyword(Keyword::Save)),
          Symbol::Term(TokenKind::Keyword(Keyword::Stop)),
          Symbol::Term(TokenKind::Keyword(Keyword::Swap)),
          Symbol::Term(TokenKind::Keyword(Keyword::System)),
          Symbol::Term(TokenKind::Keyword(Keyword::Text)),
          Symbol::Term(TokenKind::Keyword(Keyword::Trace)),
          Symbol::Term(TokenKind::Keyword(Keyword::Wend)),
          Symbol::Term(TokenKind::Keyword(Keyword::While)),
          Symbol::Term(TokenKind::Keyword(Keyword::Write)),
          Symbol::Term(TokenKind::Ident),
        ]
      }
    }
  }
}

impl<'a> IntoIterator for &'a SymbolSet {
  type Item = Symbol;
  type IntoIter = SymbolIter<'a>;

  fn into_iter(self) -> Self::IntoIter {
    SymbolIter {
      set: self,
      index: 0,
    }
  }
}

pub struct SymbolIter<'a> {
  set: &'a SymbolSet,
  index: u32,
}

impl<'a> Iterator for SymbolIter<'a> {
  type Item = Symbol;

  fn next(&mut self) -> Option<Symbol> {
    loop {
      if self.index >= 3 * 64 {
        return None;
      }
      let n = self.set.0[self.index as usize >> 6];
      let n = n & !((1 << (self.index & 63)) - 1);
      if n == 0 {
        self.index += 64;
        continue;
      }
      let next_index =
        (self.index >> 6 << 6) + (n & (n - 1) ^ n).trailing_zeros();
      self.index = next_index + 1;
      return Some(Symbol::from(next_index as usize));
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use pretty_assertions::assert_eq;

  #[test]
  fn iter() {
    let set = SymbolSet([
      0b100_0000000000_0001001000,
      0,
      0b1000_0000000000_0000000000_0000000000,
    ]);
    let result: Vec<_> = set.iter().collect();
    assert_eq!(
      result,
      vec![
        Symbol::Term(TokenKind::String),
        Symbol::Term(TokenKind::Punc(Punc::Gt)),
        Symbol::Term(TokenKind::Keyword(Keyword::Box)),
        Symbol::Nonterm(Nonterminal::Stmt),
      ]
    );
  }
}
