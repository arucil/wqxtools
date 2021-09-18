use super::super::ast::*;
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use std::{
  fmt::{self, Debug, Formatter},
  ops::BitOrAssign,
};

#[derive(Debug, Clone, Copy, FromPrimitive, PartialEq, Eq)]
pub enum Nonterminal {
  Expr,
  Stmt,
  Array,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Symbol {
  Nonterm(Nonterminal),
  Term(TokenKind),
}

#[derive(PartialEq, Eq)]
pub struct SymbolSet([u64; 3]);

#[derive(Debug)]
pub struct SymbolSetBackup {
  ptr: *mut SymbolSet,
  org_set: SymbolSet,
}

impl SymbolSet {
  pub const fn new() -> Self {
    Self([0; 3])
  }

  pub fn iter(&self) -> SymbolIter {
    self.into_iter()
  }

  const fn add(&mut self, i: usize) {
    self.0[i >> 6] |= 1 << (i & 63);
  }

  pub fn contains_token(&self, tok: TokenKind) -> bool {
    let i: usize = tok.into();
    (self.0[i >> 6] & (1 << (i & 63))) != 0
  }

  pub fn backup(&self) -> SymbolSetBackup {
    SymbolSetBackup {
      ptr: self as *const _ as *mut _,
      org_set: SymbolSet(self.0),
    }
  }

  pub fn dup(&self) -> Self {
    Self(self.0)
  }
}

impl BitOrAssign<Symbol> for SymbolSet {
  fn bitor_assign(&mut self, rhs: Symbol) {
    self.add(rhs.into());
  }
}

impl BitOrAssign<SymbolSet> for SymbolSet {
  fn bitor_assign(&mut self, rhs: SymbolSet) {
    self.0[0] |= rhs.0[0];
    self.0[1] |= rhs.0[1];
    self.0[2] |= rhs.0[2];
  }
}

impl Drop for SymbolSetBackup {
  fn drop(&mut self) {
    unsafe { (*self.ptr).0 = self.org_set.0 };
  }
}

impl SymbolSetBackup {
  pub fn set(&self, symbols: &mut SymbolSet) {
    symbols.0 = self.org_set.0;
  }
}

impl From<Symbol> for usize {
  fn from(sym: Symbol) -> Self {
    sym.to_usize()
  }
}

impl From<usize> for Symbol {
  fn from(k: usize) -> Self {
    if k < 150 {
      Self::Term(TokenKind::from(k))
    } else {
      Self::Nonterm(Nonterminal::from_usize(k - 150).unwrap())
    }
  }
}

impl Symbol {
  pub const fn to_usize(&self) -> usize {
    match self {
      Symbol::Term(t) => t.to_usize(),
      Symbol::Nonterm(n) => 150 + *n as usize,
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
  const EXPR_FIRST_SYMBOLS: SymbolSet = {
    let mut set = SymbolSet::new();
    set.add(Symbol::Term(TokenKind::Float).to_usize());
    set.add(Symbol::Term(TokenKind::Label).to_usize());
    set.add(Symbol::Term(TokenKind::String).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Inkey)).to_usize());
    set.add(Symbol::Term(TokenKind::Punc(Punc::Plus)).to_usize());
    set.add(Symbol::Term(TokenKind::Punc(Punc::Minus)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Not)).to_usize());
    set.add(Symbol::Term(TokenKind::Punc(Punc::LParen)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Fn)).to_usize());
    set.add(Symbol::Term(TokenKind::SysFunc(SysFuncKind::Abs)).to_usize());
    set.add(Symbol::Term(TokenKind::SysFunc(SysFuncKind::Asc)).to_usize());
    set.add(Symbol::Term(TokenKind::SysFunc(SysFuncKind::Atn)).to_usize());
    set.add(Symbol::Term(TokenKind::SysFunc(SysFuncKind::Chr)).to_usize());
    set.add(Symbol::Term(TokenKind::SysFunc(SysFuncKind::Cos)).to_usize());
    set.add(Symbol::Term(TokenKind::SysFunc(SysFuncKind::Cvi)).to_usize());
    set.add(Symbol::Term(TokenKind::SysFunc(SysFuncKind::Cvs)).to_usize());
    set.add(Symbol::Term(TokenKind::SysFunc(SysFuncKind::Eof)).to_usize());
    set.add(Symbol::Term(TokenKind::SysFunc(SysFuncKind::Exp)).to_usize());
    set.add(Symbol::Term(TokenKind::SysFunc(SysFuncKind::Int)).to_usize());
    set.add(Symbol::Term(TokenKind::SysFunc(SysFuncKind::Left)).to_usize());
    set.add(Symbol::Term(TokenKind::SysFunc(SysFuncKind::Len)).to_usize());
    set.add(Symbol::Term(TokenKind::SysFunc(SysFuncKind::Lof)).to_usize());
    set.add(Symbol::Term(TokenKind::SysFunc(SysFuncKind::Log)).to_usize());
    set.add(Symbol::Term(TokenKind::SysFunc(SysFuncKind::Mid)).to_usize());
    set.add(Symbol::Term(TokenKind::SysFunc(SysFuncKind::Mki)).to_usize());
    set.add(Symbol::Term(TokenKind::SysFunc(SysFuncKind::Mks)).to_usize());
    set.add(Symbol::Term(TokenKind::SysFunc(SysFuncKind::Peek)).to_usize());
    set.add(Symbol::Term(TokenKind::SysFunc(SysFuncKind::Pos)).to_usize());
    set.add(Symbol::Term(TokenKind::SysFunc(SysFuncKind::Right)).to_usize());
    set.add(Symbol::Term(TokenKind::SysFunc(SysFuncKind::Rnd)).to_usize());
    set.add(Symbol::Term(TokenKind::SysFunc(SysFuncKind::Sgn)).to_usize());
    set.add(Symbol::Term(TokenKind::SysFunc(SysFuncKind::Sin)).to_usize());
    set.add(Symbol::Term(TokenKind::SysFunc(SysFuncKind::Sqr)).to_usize());
    set.add(Symbol::Term(TokenKind::SysFunc(SysFuncKind::Str)).to_usize());
    set.add(Symbol::Term(TokenKind::SysFunc(SysFuncKind::Tan)).to_usize());
    set.add(Symbol::Term(TokenKind::SysFunc(SysFuncKind::Val)).to_usize());
    set.add(Symbol::Term(TokenKind::Ident).to_usize());
    set
  };

  const STMT_FIRST_SYMBOLS: SymbolSet = {
    let mut set = SymbolSet::new();
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Auto)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Beep)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Box)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Call)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Circle)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Clear)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Close)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Cls)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Cont)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Copy)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Data)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Def)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Del)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Dim)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Draw)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Edit)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Ellipse)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::End)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Field)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Files)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Flash)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::For)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Get)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Gosub)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Goto)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Graph)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::If)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Inkey)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Input)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Inverse)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Kill)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Let)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Line)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::List)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Load)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Locate)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Lset)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::New)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Next)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Normal)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Notrace)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::On)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Open)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Play)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Poke)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Pop)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Print)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Put)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Read)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Rem)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Rename)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Restore)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Return)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Rset)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Run)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Save)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Stop)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Swap)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::System)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Text)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Trace)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Wend)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::While)).to_usize());
    set.add(Symbol::Term(TokenKind::Keyword(Keyword::Write)).to_usize());
    set.add(Symbol::Term(TokenKind::Ident).to_usize());
    set
  };

  const ARRAY_FIRST_SYMBOLS: SymbolSet = {
    let mut set = SymbolSet::new();
    set.add(Symbol::Term(TokenKind::Ident).to_usize());
    set
  };

  pub fn first_symbols(&self) -> SymbolSet {
    match self {
      Self::Expr => Self::EXPR_FIRST_SYMBOLS,
      Self::Stmt => Self::STMT_FIRST_SYMBOLS,
      Self::Array => Self::ARRAY_FIRST_SYMBOLS,
    }
  }
}

impl Debug for SymbolSet {
  fn fmt(&self, f: &mut Formatter) -> fmt::Result {
    f.debug_list().entries(self.iter()).finish()
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
      let n = n & !((1u64 << (self.index & 63)) - 1);
      if n == 0 {
        self.index = (self.index + 64) & !63;
        continue;
      }
      let next_index = (self.index & !63) + (n & (n - 1) ^ n).trailing_zeros();
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
      0b100_0000_0000000000_0001001000,
      0,
      0b10_00_0000000000_0100000000,
    ]);
    let result: Vec<_> = set.iter().collect();
    assert_eq!(
      result,
      vec![
        Symbol::Term(TokenKind::String),
        Symbol::Term(TokenKind::Punc(Punc::Gt)),
        Symbol::Term(TokenKind::Keyword(Keyword::Box)),
        Symbol::Term(TokenKind::SysFunc(SysFuncKind::Val)),
        Symbol::Nonterm(Nonterminal::Stmt),
      ]
    );
  }

  #[test]
  fn build_symbol_set() {
    let mut set = SymbolSet::new();
    set |= Symbol::Term(TokenKind::String);
    set |= Symbol::Term(TokenKind::Punc(Punc::Gt));
    set |= Symbol::Term(TokenKind::Keyword(Keyword::Box));
    set |= Symbol::Term(TokenKind::SysFunc(SysFuncKind::Val));
    set |= Symbol::Nonterm(Nonterminal::Stmt);
    assert_eq!(
      set,
      SymbolSet([
        0b100_0000_0000000000_0001001000,
        0,
        0b10_00_0000000000_0100000000,
      ])
    );
  }

  #[test]
  fn contains_token() {
    let set = SymbolSet([
      0b100_0000_0000000000_0001001000,
      0,
      0b10_00_0000000000_0100000000,
    ]);
    assert!(set.contains_token(TokenKind::Punc(Punc::Gt)));
    assert!(!set.contains_token(TokenKind::Punc(Punc::Colon)));
  }
}
