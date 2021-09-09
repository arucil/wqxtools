use std::str::FromStr;

use super::expr::SysFuncKind;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
  Ident,
  Number,
  String,
  Punc(Punc),
  Keyword(Keyword),
  SysFunc(SysFuncKind),
  Eof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
  Auto,
  Beep,
  Box,
  Call,
  Circle,
  Clear,
  Close,
  Cls,
  Cont,
  Copy,
  Data,
  Def,
  Del,
  Dim,
  Draw,
  Edit,
  Ellipse,
  End,
  Field,
  Files,
  Flash,
  For,
  Get,
  Gosub,
  Goto,
  Graph,
  If,
  Inkey,
  Input,
  Inverse,
  Kill,
  Let,
  Line,
  List,
  Load,
  Locate,
  Lset,
  New,
  Next,
  Normal,
  Notrace,
  On,
  Open,
  Play,
  Poke,
  Pop,
  Print,
  Put,
  Read,
  Rem,
  Rename,
  Restore,
  Return,
  Rset,
  Run,
  Save,
  Stop,
  Swap,
  System,
  Text,
  Trace,
  Wend,
  While,
  Write,

  Then,
  Else,
  To,
  Step,
  Fn,
  And,
  Or,
  Not,

  Sleep,
  Paint,
  Fputc,
  Fread,
  Fwrite,
  Fseek,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Punc {
  Eq,
  Lt,
  Gt,
  Plus,
  Minus,
  Times,
  Slash,
  Caret,
  Colon,
  LParen,
  RParen,
  Semi,
  Comma,
  Hash,
}

impl FromStr for Keyword {
  type Err = ();
  fn from_str(s: &str) -> Result<Self, ()> {
    match s {
      "auto" => Ok(Self::Auto),
      "beep" => Ok(Self::Beep),
      "box" => Ok(Self::Box),
      "call" => Ok(Self::Call),
      "circle" => Ok(Self::Circle),
      "clear" => Ok(Self::Clear),
      "close" => Ok(Self::Close),
      "cls" => Ok(Self::Cls),
      "cont" => Ok(Self::Cont),
      "copy" => Ok(Self::Copy),
      "data" => Ok(Self::Data),
      "def" => Ok(Self::Def),
      "del" => Ok(Self::Del),
      "dim" => Ok(Self::Dim),
      "draw" => Ok(Self::Draw),
      "edit" => Ok(Self::Edit),
      "ellipse" => Ok(Self::Ellipse),
      "end" => Ok(Self::End),
      "field" => Ok(Self::Field),
      "files" => Ok(Self::Files),
      "flash" => Ok(Self::Flash),
      "for" => Ok(Self::For),
      "get" => Ok(Self::Get),
      "gosub" => Ok(Self::Gosub),
      "goto" => Ok(Self::Goto),
      "graph" => Ok(Self::Graph),
      "if" => Ok(Self::If),
      "inkey$" => Ok(Self::Inkey),
      "input" => Ok(Self::Input),
      "inverse" => Ok(Self::Inverse),
      "kill" => Ok(Self::Kill),
      "let" => Ok(Self::Let),
      "line" => Ok(Self::Line),
      "list" => Ok(Self::List),
      "load" => Ok(Self::Load),
      "locate" => Ok(Self::Locate),
      "lset" => Ok(Self::Lset),
      "new" => Ok(Self::New),
      "next" => Ok(Self::Next),
      "normal" => Ok(Self::Normal),
      "notrace" => Ok(Self::Notrace),
      "on" => Ok(Self::On),
      "open" => Ok(Self::Open),
      "play" => Ok(Self::Play),
      "poke" => Ok(Self::Poke),
      "pop" => Ok(Self::Pop),
      "print" => Ok(Self::Print),
      "put" => Ok(Self::Put),
      "read" => Ok(Self::Read),
      "rem" => Ok(Self::Rem),
      "rename" => Ok(Self::Rename),
      "restore" => Ok(Self::Restore),
      "return" => Ok(Self::Return),
      "rset" => Ok(Self::Rset),
      "run" => Ok(Self::Run),
      "save" => Ok(Self::Save),
      "stop" => Ok(Self::Stop),
      "swap" => Ok(Self::Swap),
      "system" => Ok(Self::System),
      "text" => Ok(Self::Text),
      "trace" => Ok(Self::Trace),
      "wend" => Ok(Self::Wend),
      "while" => Ok(Self::While),
      "write" => Ok(Self::Write),

      "then" => Ok(Self::Then),
      "else" => Ok(Self::Else),
      "to" => Ok(Self::To),
      "step" => Ok(Self::Step),
      "fn" => Ok(Self::Fn),
      "and" => Ok(Self::And),
      "or" => Ok(Self::Or),
      "not" => Ok(Self::Not),

      "sleep" => Ok(Self::Sleep),
      "paint" => Ok(Self::Paint),
      "fputc" => Ok(Self::Fputc),
      "fread" => Ok(Self::Fread),
      "fwrite" => Ok(Self::Fwrite),
      "fseek" => Ok(Self::Fseek),
      _ => Err(()),
    }
  }
}

impl From<u8> for Punc {
  fn from(c: u8) -> Self {
    match c {
      b'=' => Self::Eq,
      b'<' => Self::Lt,
      b'>' => Self::Gt,
      b'+' => Self::Plus,
      b'-' => Self::Minus,
      b'*' => Self::Times,
      b'/' => Self::Slash,
      b'^' => Self::Caret,
      b':' => Self::Colon,
      b'(' => Self::LParen,
      b')' => Self::RParen,
      b';' => Self::Semi,
      b',' => Self::Comma,
      b'#' => Self::Hash,
      _ => panic!("invalid char {}", c),
    }
  }
}
