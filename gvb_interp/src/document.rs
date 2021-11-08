use std::fs;
use std::io;
use std::path::Path;

use crate::ast::{Program, ProgramLine};
use crate::compiler::compile_prog;
use crate::machine::EmojiStyle;
use crate::machine::MachineProps;
use crate::parser::{parse_line, ParseResult};
use crate::{CodeGen, Diagnostic};

mod binary;

const DEFAULT_TEXT: &str = "10 ";

pub struct Document {
  base_addr: u16,
  emoji_style: EmojiStyle,
  machine_props: MachineProps,
  text: String,
  lines: Vec<DocLine>,
  version: DocVer,
  compile_cache: Option<CompileCache>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DocVer(u64);

struct CompileCache {
  version: DocVer,
  codegen: CodeGen,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DocLine<T = ParseResult<ProgramLine>> {
  line_start: usize,
  parsed: Option<T>,
}

#[derive(Debug, Clone)]
pub struct LineDiagnosis {
  pub line_start: usize,
  pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug)]
pub enum LoadDocumentError {
  Io(io::Error),
  UnknownExt(Option<String>),
  LoadBas(binary::LoadError<usize>),
  LoadTxt(binary::LoadError<(usize, usize)>),
}

#[derive(Debug)]
pub enum SaveDocumentError {
  Io(io::Error),
  InvalidExt(Option<String>),
  Save(binary::SaveError),
}

#[derive(Debug)]
pub struct Edit<'a> {
  pub pos: usize,
  pub kind: EditKind<'a>,
}

#[derive(Debug)]
pub enum EditKind<'a> {
  Insert(&'a str),
  Delete(usize),
}

impl From<io::Error> for LoadDocumentError {
  fn from(err: io::Error) -> Self {
    Self::Io(err)
  }
}

impl From<binary::LoadError<usize>> for LoadDocumentError {
  fn from(err: binary::LoadError<usize>) -> Self {
    Self::LoadBas(err)
  }
}

impl From<binary::LoadError<(usize, usize)>> for LoadDocumentError {
  fn from(err: binary::LoadError<(usize, usize)>) -> Self {
    Self::LoadTxt(err)
  }
}

impl From<io::Error> for SaveDocumentError {
  fn from(err: io::Error) -> Self {
    Self::Io(err)
  }
}

impl From<binary::SaveError> for SaveDocumentError {
  fn from(err: binary::SaveError) -> Self {
    Self::Save(err)
  }
}

impl Document {
  pub fn new() -> Self {
    Self {
      base_addr: binary::DEFAULT_BASE_ADDR,
      emoji_style: EmojiStyle::New,
      machine_props: crate::machine::MACHINES
        [EmojiStyle::New.default_machine_name()]
      .clone(),
      text: DEFAULT_TEXT.to_owned(),
      lines: text_to_doc_lines(DEFAULT_TEXT),
      version: DocVer(0),
      compile_cache: None,
    }
  }

  /// Load a `.BAS` or `.txt` file.
  pub fn load<P>(path: P) -> Result<Self, LoadDocumentError>
  where
    P: AsRef<Path>,
  {
    let path = path.as_ref();
    let ext = path.extension().map(|ext| ext.to_ascii_lowercase());
    let is_bas = if let Some(ext) = ext {
      match ext.to_str() {
        Some("bas") => true,
        Some("txt") => false,
        ext => {
          return Err(LoadDocumentError::UnknownExt(
            ext.map(|ext| ext.to_owned()),
          ))
        }
      }
    } else {
      return Err(LoadDocumentError::UnknownExt(None));
    };

    let data = fs::read(path)?;

    let mut doc = if is_bas {
      binary::load_bas(&data, None)?
    } else {
      binary::load_txt(&data, None)?
    };

    let mut emoji_style = doc.guessed_emoji_style;

    let machine_props;
    if let Some(props) = detect_machine_props(&doc.text)
      .and_then(|p| p.ok())
      .cloned()
    {
      emoji_style = props.emoji_style;
      doc = if is_bas {
        binary::load_bas(&data, Some(emoji_style))?
      } else {
        binary::load_txt(&data, Some(emoji_style))?
      };
      machine_props = props;
    } else {
      machine_props =
        crate::machine::MACHINES[emoji_style.default_machine_name()].clone();
    }

    let lines = text_to_doc_lines(&doc.text);

    Ok(Document {
      base_addr: doc.base_addr,
      emoji_style,
      machine_props,
      text: doc.text,
      lines,
      version: DocVer(0),
      compile_cache: None,
    })
  }

  /// Save to a `.BAS` or `.txt` file.
  pub fn save<P>(&self, path: P) -> Result<(), SaveDocumentError>
  where
    P: AsRef<Path>,
  {
    let path = path.as_ref();
    let ext = path.extension().map(|ext| ext.to_ascii_lowercase());
    let is_bas = if let Some(ext) = ext {
      match ext.to_str() {
        Some("bas") => true,
        Some("txt") => false,
        ext => {
          return Err(SaveDocumentError::InvalidExt(
            ext.map(|ext| ext.to_owned()),
          ))
        }
      }
    } else {
      return Err(SaveDocumentError::InvalidExt(None));
    };

    let data = if is_bas {
      binary::save_bas(self.text(), self.emoji_style, self.base_addr)?
    } else {
      binary::save_txt(self.text(), self.emoji_style)?
    };

    fs::write(path, data)?;

    Ok(())
  }

  pub fn diagnostics(&mut self) -> Vec<LineDiagnosis> {
    let mut prog = Program {
      lines: Vec::with_capacity(self.lines.len()),
    };
    for i in 0..self.lines.len() {
      if let Some(p) = self.lines[i].parsed.as_ref().cloned() {
        prog.lines.push(p);
      } else {
        let start = self.lines[i].line_start;
        let end = self
          .lines
          .get(i + 1)
          .map_or(self.text.len(), |line| line.line_start);
        let p = parse_line(&self.text[start..end]).0;
        self.lines[i].parsed = Some(p.clone());
        prog.lines.push(p);
      }
    }
    let text = self.text();
    let mut codegen = CodeGen::new(self.emoji_style);
    compile_prog(text, &mut prog, &mut codegen);
    self.compile_cache = Some(CompileCache {
      version: self.version,
      codegen,
    });

    prog
      .lines
      .into_iter()
      .zip(&self.lines)
      .map(|(line, doc_line)| LineDiagnosis {
        line_start: doc_line.line_start,
        diagnostics: line.diagnostics,
      })
      .collect()
  }

  pub fn apply_edit(&mut self, edit: Edit) {
    apply_edit(&mut self.text, &mut self.lines, edit);
    self.version.0 += 1;
  }

  pub fn text(&self) -> String {
    // TODO &str
    self.text.clone()
  }

  pub fn machine_name(&self) -> &'static str {
    self.machine_props.name
  }

  pub fn sync_machine(&mut self) {
    if let Some(Ok(props)) = detect_machine_props(self.text()) {
      self.machine_props = props.clone();
    } else {
      self.machine_props = crate::machine::MACHINES
        [self.emoji_style.default_machine_name()]
      .clone();
    }
    todo!("set emoji_style, reload text")
  }

  pub fn set_machine_name(&self, name: &str) -> bool {
    todo!("set emoji_style, reload text, set machine_props")
  }
}

fn detect_machine_props(
  text: impl AsRef<str>,
) -> Option<Result<&'static MachineProps, ()>> {
  let last_line = text.as_ref().lines().last().unwrap();
  if let Some(start) = last_line.rfind('{') {
    let first_line = &last_line[start + 1..];
    if let Some(end) = first_line.find('}') {
      let name = first_line[..end].trim().to_ascii_uppercase();
      if !name.is_empty() {
        match crate::machine::MACHINES.get(&name) {
          Some(props) => return Some(Ok(props)),
          None => return Some(Err(())),
        }
      }
    }
  }
  None
}

fn text_to_doc_lines(text: impl AsRef<str>) -> Vec<DocLine> {
  let text = text.as_ref();
  let mut lines: Vec<DocLine> = vec![];
  let mut line_start = 0;
  while let Some(eol) = text[line_start..].find('\n') {
    lines.push(DocLine {
      line_start,
      parsed: None,
    });
    line_start += eol + 1;
  }
  lines.push(DocLine {
    line_start,
    parsed: None,
  });
  lines
}

fn apply_edit(text: &mut String, lines: &mut Vec<DocLine>, edit: Edit) {
  let mut i = 0;
  while i < lines.len() - 1 && edit.pos >= lines[i + 1].line_start {
    i += 1;
  }

  match edit.kind {
    EditKind::Insert(str) => {
      text.insert_str(edit.pos, &str);
      lines[i].parsed = None;

      if str.contains('\n') {
        let line_start = lines[i].line_start;
        let line_end = lines
          .get(i + 1)
          .map_or(text.len(), |line| line.line_start + str.len());
        let line = &text[line_start..line_end];
        let mut new_lines = text_to_doc_lines(line);
        if line.ends_with('\n') {
          new_lines.pop();
        }
        let num_new_lines = new_lines.len();
        lines.splice(i..i + 1, new_lines);
        for i in i..i + num_new_lines {
          lines[i].line_start += line_start;
        }
        i += num_new_lines - 1;
      }

      for line in &mut lines[i + 1..] {
        line.line_start += str.len();
      }
    }
    EditKind::Delete(del_len) => {
      lines[i].parsed = None;
      let deleted_part = &text[edit.pos..edit.pos + del_len];
      if deleted_part.contains('\n') {
        let deleted_lines = deleted_part.matches('\n').count();
        let line_start = lines[i].line_start;
        if edit.pos == line_start && deleted_part.ends_with('\n') {
          lines.drain(i..i + deleted_lines);
          lines[i].line_start = line_start;
        } else {
          lines.drain(i + 1..i + deleted_lines + 1);
        }
      }
      for line in &mut lines[i + 1..] {
        line.line_start -= del_len;
      }
      text.replace_range(edit.pos..edit.pos + del_len, "");
    }
  }

  if lines.is_empty()
    || text.ends_with('\n') && lines.last().unwrap().line_start < text.len()
  {
    lines.push(DocLine {
      line_start: text.len(),
      parsed: None,
    });
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::ast::Eol;
  use id_arena::Arena;
  use pretty_assertions::assert_eq;
  use smallvec::SmallVec;

  fn doc_line(line_start: usize) -> DocLine<()> {
    DocLine {
      line_start,
      parsed: Some(()),
    }
  }

  fn dirty_doc_line(line_start: usize) -> DocLine<()> {
    DocLine {
      line_start,
      parsed: None,
    }
  }

  fn doc_lines(lines: Vec<DocLine>) -> Vec<DocLine<()>> {
    lines
      .into_iter()
      .map(|line| DocLine {
        line_start: line.line_start,
        parsed: line.parsed.map(|_| ()),
      })
      .collect()
  }

  fn dummy_parsed() -> ParseResult<ProgramLine> {
    ParseResult {
      stmt_arena: Arena::new(),
      expr_arena: Arena::new(),
      content: ProgramLine {
        source_len: 0,
        label: None,
        stmts: SmallVec::new(),
        eol: Eol::CrLf,
      },
      diagnostics: vec![],
    }
  }

  fn make_lines(text: &str) -> (String, Vec<DocLine>) {
    let text = text.replace('\n', "\r\n");
    let mut lines = text_to_doc_lines(&text);
    for line in &mut lines {
      line.parsed = Some(dummy_parsed());
    }
    (text, lines)
  }

  const INPUT: &str = "\
abcd
efg
hijklm
no";

  #[test]
  fn delete_middle() {
    let (mut text, mut lines) = make_lines(INPUT);
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 13,
        kind: EditKind::Delete(3),
      },
    );

    assert_eq!(text.as_str(), "abcd\r\nefg\r\nhim\r\nno");

    assert_eq!(
      doc_lines(lines),
      vec![doc_line(0), doc_line(6), dirty_doc_line(11), doc_line(16),]
    );
  }

  #[test]
  fn delete_to_end() {
    let (mut text, mut lines) = make_lines(INPUT);
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 1,
        kind: EditKind::Delete(3),
      },
    );

    assert_eq!(text.as_str(), "a\r\nefg\r\nhijklm\r\nno");

    assert_eq!(
      doc_lines(lines),
      vec![dirty_doc_line(0), doc_line(3), doc_line(8), doc_line(16),]
    );
  }

  #[test]
  fn delete_to_newline() {
    let (mut text, mut lines) = make_lines(INPUT);
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 1,
        kind: EditKind::Delete(5),
      },
    );

    assert_eq!(text.as_str(), "aefg\r\nhijklm\r\nno");

    assert_eq!(
      doc_lines(lines),
      vec![dirty_doc_line(0), doc_line(6), doc_line(14),]
    );
  }

  #[test]
  fn delete_newline() {
    let (mut text, mut lines) = make_lines(INPUT);
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 4,
        kind: EditKind::Delete(2),
      },
    );

    assert_eq!(text.as_str(), "abcdefg\r\nhijklm\r\nno");

    assert_eq!(
      doc_lines(lines),
      vec![dirty_doc_line(0), doc_line(9), doc_line(17),]
    );
  }

  #[test]
  fn delete_join_lines() {
    let (mut text, mut lines) = make_lines(INPUT);
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 3,
        kind: EditKind::Delete(4),
      },
    );

    assert_eq!(text.as_str(), "abcfg\r\nhijklm\r\nno");

    assert_eq!(
      doc_lines(lines),
      vec![dirty_doc_line(0), doc_line(7), doc_line(15),]
    );
  }

  #[test]
  fn delete_first_line() {
    let (mut text, mut lines) = make_lines(INPUT);
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 0,
        kind: EditKind::Delete(6),
      },
    );

    assert_eq!(text.as_str(), "efg\r\nhijklm\r\nno");

    assert_eq!(
      doc_lines(lines),
      vec![doc_line(0), doc_line(5), doc_line(13),]
    );
  }

  #[test]
  fn delete_last_line() {
    let (mut text, mut lines) = make_lines(INPUT);
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 19,
        kind: EditKind::Delete(2),
      },
    );

    assert_eq!(text.as_str(), "abcd\r\nefg\r\nhijklm\r\n");

    assert_eq!(
      doc_lines(lines),
      vec![doc_line(0), doc_line(6), doc_line(11), dirty_doc_line(19),]
    );
  }

  #[test]
  fn delete_middle_line() {
    let (mut text, mut lines) = make_lines(INPUT);
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 11,
        kind: EditKind::Delete(8),
      },
    );

    assert_eq!(text.as_str(), "abcd\r\nefg\r\nno");

    assert_eq!(
      doc_lines(lines),
      vec![doc_line(0), doc_line(6), doc_line(11),]
    );
  }

  #[test]
  fn delete_first_multiple_lines() {
    let (mut text, mut lines) = make_lines(INPUT);
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 0,
        kind: EditKind::Delete(19),
      },
    );

    assert_eq!(text.as_str(), "no");

    assert_eq!(doc_lines(lines), vec![doc_line(0)]);
  }

  #[test]
  fn delete_last_multiple_lines() {
    let (mut text, mut lines) = make_lines(INPUT);
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 6,
        kind: EditKind::Delete(15),
      },
    );

    assert_eq!(text.as_str(), "abcd\r\n");

    assert_eq!(doc_lines(lines), vec![doc_line(0), dirty_doc_line(6)]);
  }

  #[test]
  fn delete_middle_multiple_lines() {
    let (mut text, mut lines) = make_lines(INPUT);
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 6,
        kind: EditKind::Delete(13),
      },
    );

    assert_eq!(text.as_str(), "abcd\r\nno");

    assert_eq!(doc_lines(lines), vec![doc_line(0), doc_line(6)]);
  }

  #[test]
  fn delete_across_multiple_lines() {
    let (mut text, mut lines) = make_lines(INPUT);
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 2,
        kind: EditKind::Delete(11),
      },
    );

    assert_eq!(text.as_str(), "abjklm\r\nno");

    assert_eq!(doc_lines(lines), vec![dirty_doc_line(0), doc_line(8)]);
  }

  #[test]
  fn delete_across_multiple_lines_until_newline() {
    let (mut text, mut lines) = make_lines(INPUT);
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 2,
        kind: EditKind::Delete(15),
      },
    );

    assert_eq!(text.as_str(), "ab\r\nno");

    assert_eq!(doc_lines(lines), vec![dirty_doc_line(0), doc_line(4)]);
  }

  #[test]
  fn delete_across_multiple_lines_newline() {
    let (mut text, mut lines) = make_lines(INPUT);
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 2,
        kind: EditKind::Delete(17),
      },
    );

    assert_eq!(text.as_str(), "abno");

    assert_eq!(doc_lines(lines), vec![dirty_doc_line(0)]);
  }

  #[test]
  fn delete_all() {
    let (mut text, mut lines) = make_lines(INPUT);
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 0,
        kind: EditKind::Delete(21),
      },
    );

    assert_eq!(text.as_str(), "");

    assert_eq!(doc_lines(lines), vec![dirty_doc_line(0),]);
  }

  #[test]
  fn insert_middle() {
    let (mut text, mut lines) = make_lines(INPUT);
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 13,
        kind: EditKind::Insert("123"),
      },
    );

    assert_eq!(text.as_str(), "abcd\r\nefg\r\nhi123jklm\r\nno");

    assert_eq!(
      doc_lines(lines),
      vec![doc_line(0), doc_line(6), dirty_doc_line(11), doc_line(22)]
    );
  }

  #[test]
  fn insert_into_empty() {
    let (mut text, mut lines) = make_lines("");
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 0,
        kind: EditKind::Insert("123"),
      },
    );

    assert_eq!(text.as_str(), "123");

    assert_eq!(doc_lines(lines), vec![dirty_doc_line(0)]);
  }

  #[test]
  fn insert_after_newline() {
    let (mut text, mut lines) = make_lines(INPUT);
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 11,
        kind: EditKind::Insert("123"),
      },
    );

    assert_eq!(text.as_str(), "abcd\r\nefg\r\n123hijklm\r\nno");

    assert_eq!(
      doc_lines(lines),
      vec![doc_line(0), doc_line(6), dirty_doc_line(11), doc_line(22)]
    );
  }

  #[test]
  fn insert_at_start() {
    let (mut text, mut lines) = make_lines(INPUT);
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 0,
        kind: EditKind::Insert("123"),
      },
    );

    assert_eq!(text.as_str(), "123abcd\r\nefg\r\nhijklm\r\nno");

    assert_eq!(
      doc_lines(lines),
      vec![dirty_doc_line(0), doc_line(9), doc_line(14), doc_line(22),]
    );
  }

  #[test]
  fn insert_before_newline() {
    let (mut text, mut lines) = make_lines(INPUT);
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 17,
        kind: EditKind::Insert("123"),
      },
    );

    assert_eq!(text.as_str(), "abcd\r\nefg\r\nhijklm123\r\nno");

    assert_eq!(
      doc_lines(lines),
      vec![doc_line(0), doc_line(6), dirty_doc_line(11), doc_line(22)]
    );
  }

  #[test]
  fn insert_at_end() {
    let (mut text, mut lines) = make_lines(&INPUT[..INPUT.len() - 2]);
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 19,
        kind: EditKind::Insert("no"),
      },
    );

    assert_eq!(text.as_str(), "abcd\r\nefg\r\nhijklm\r\nno");

    assert_eq!(
      doc_lines(lines),
      vec![doc_line(0), doc_line(6), doc_line(11), dirty_doc_line(19),]
    );
  }

  #[test]
  fn insert_multiple_lines_at_start() {
    let (mut text, mut lines) = make_lines(INPUT);
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 0,
        kind: EditKind::Insert("123\r\n45\r\n"),
      },
    );

    assert_eq!(text.as_str(), "123\r\n45\r\nabcd\r\nefg\r\nhijklm\r\nno");

    assert_eq!(
      doc_lines(lines),
      vec![
        dirty_doc_line(0),
        dirty_doc_line(5),
        dirty_doc_line(9),
        doc_line(15),
        doc_line(20),
        doc_line(28),
      ]
    );
  }

  #[test]
  fn insert_multiple_lines_into_empty() {
    let (mut text, mut lines) = make_lines("");
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 0,
        kind: EditKind::Insert("abcd\r\nefg\r\nhijklm\r\nno"),
      },
    );

    assert_eq!(text.as_str(), "abcd\r\nefg\r\nhijklm\r\nno");

    assert_eq!(
      doc_lines(lines),
      vec![
        dirty_doc_line(0),
        dirty_doc_line(6),
        dirty_doc_line(11),
        dirty_doc_line(19)
      ]
    );
  }

  #[test]
  fn insert_multiple_lines_with_newline_into_empty() {
    let (mut text, mut lines) = make_lines("");
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 0,
        kind: EditKind::Insert("abcd\r\nefg\r\nhijklm\r\nno\r\n"),
      },
    );

    assert_eq!(text.as_str(), "abcd\r\nefg\r\nhijklm\r\nno\r\n");

    assert_eq!(
      doc_lines(lines),
      vec![
        dirty_doc_line(0),
        dirty_doc_line(6),
        dirty_doc_line(11),
        dirty_doc_line(19),
        dirty_doc_line(23),
      ]
    );
  }

  #[test]
  fn insert_multiple_lines() {
    let (mut text, mut lines) = make_lines(INPUT);
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 13,
        kind: EditKind::Insert("123\r\n45\r\n6789"),
      },
    );

    assert_eq!(
      text.as_str(),
      "abcd\r\nefg\r\nhi123\r\n45\r\n6789jklm\r\nno"
    );

    assert_eq!(
      doc_lines(lines),
      vec![
        doc_line(0),
        doc_line(6),
        dirty_doc_line(11),
        dirty_doc_line(18),
        dirty_doc_line(22),
        doc_line(32)
      ]
    );
  }

  #[test]
  fn insert_multiple_lines_after_newline() {
    let (mut text, mut lines) = make_lines(INPUT);
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 11,
        kind: EditKind::Insert("123\r\n45\r\n6789"),
      },
    );

    assert_eq!(
      text.as_str(),
      "abcd\r\nefg\r\n123\r\n45\r\n6789hijklm\r\nno"
    );

    assert_eq!(
      doc_lines(lines),
      vec![
        doc_line(0),
        doc_line(6),
        dirty_doc_line(11),
        dirty_doc_line(16),
        dirty_doc_line(20),
        doc_line(32)
      ]
    );
  }

  #[test]
  fn insert_multiple_lines_and_newline_after_newline() {
    let (mut text, mut lines) = make_lines(INPUT);
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 11,
        kind: EditKind::Insert("123\r\n45\r\n6789\r\n"),
      },
    );

    assert_eq!(
      text.as_str(),
      "abcd\r\nefg\r\n123\r\n45\r\n6789\r\nhijklm\r\nno"
    );

    assert_eq!(
      doc_lines(lines),
      vec![
        doc_line(0),
        doc_line(6),
        dirty_doc_line(11),
        dirty_doc_line(16),
        dirty_doc_line(20),
        dirty_doc_line(26),
        doc_line(34)
      ]
    );
  }

  #[test]
  fn insert_multiple_lines_before_newline() {
    let (mut text, mut lines) = make_lines("abcd\nefg\nhijklm\nno\n");
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 17,
        kind: EditKind::Insert("123\r\n45\r\n6789"),
      },
    );

    assert_eq!(
      text.as_str(),
      "abcd\r\nefg\r\nhijklm123\r\n45\r\n6789\r\nno\r\n"
    );

    assert_eq!(
      doc_lines(lines),
      vec![
        doc_line(0),
        doc_line(6),
        dirty_doc_line(11),
        dirty_doc_line(22),
        dirty_doc_line(26),
        doc_line(32),
        doc_line(36)
      ]
    );
  }

  #[test]
  fn insert_multiple_lines_and_newline_before_newline() {
    let (mut text, mut lines) = make_lines(INPUT);
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 17,
        kind: EditKind::Insert("123\r\n45\r\n6789\r\n"),
      },
    );

    assert_eq!(
      text.as_str(),
      "abcd\r\nefg\r\nhijklm123\r\n45\r\n6789\r\n\r\nno"
    );

    assert_eq!(
      doc_lines(lines),
      vec![
        doc_line(0),
        doc_line(6),
        dirty_doc_line(11),
        dirty_doc_line(22),
        dirty_doc_line(26),
        dirty_doc_line(32),
        doc_line(34)
      ]
    );
  }

  #[test]
  fn insert_multiple_lines_at_end() {
    let (mut text, mut lines) = make_lines(INPUT);
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 21,
        kind: EditKind::Insert("123\r\n45\r\n6789"),
      },
    );

    assert_eq!(
      text.as_str(),
      "abcd\r\nefg\r\nhijklm\r\nno123\r\n45\r\n6789"
    );

    assert_eq!(
      doc_lines(lines),
      vec![
        doc_line(0),
        doc_line(6),
        doc_line(11),
        dirty_doc_line(19),
        dirty_doc_line(26),
        dirty_doc_line(30),
      ]
    );
  }

  #[test]
  fn insert_multiple_lines_and_newline_at_end() {
    let (mut text, mut lines) = make_lines(INPUT);
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 21,
        kind: EditKind::Insert("123\r\n45\r\n6789\r\n"),
      },
    );

    assert_eq!(
      text.as_str(),
      "abcd\r\nefg\r\nhijklm\r\nno123\r\n45\r\n6789\r\n"
    );

    assert_eq!(
      doc_lines(lines),
      vec![
        doc_line(0),
        doc_line(6),
        doc_line(11),
        dirty_doc_line(19),
        dirty_doc_line(26),
        dirty_doc_line(30),
        dirty_doc_line(36),
      ]
    );
  }

  #[test]
  fn insert_multiple_lines_into_last_line() {
    let (mut text, mut lines) = make_lines(INPUT);
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 20,
        kind: EditKind::Insert("123\r\n45\r\n6789"),
      },
    );

    assert_eq!(
      text.as_str(),
      "abcd\r\nefg\r\nhijklm\r\nn123\r\n45\r\n6789o"
    );

    assert_eq!(
      doc_lines(lines),
      vec![
        doc_line(0),
        doc_line(6),
        doc_line(11),
        dirty_doc_line(19),
        dirty_doc_line(25),
        dirty_doc_line(29),
      ]
    );
  }

  #[test]
  fn insert_multiple_lines_and_newline_into_last_line() {
    let (mut text, mut lines) = make_lines(INPUT);
    apply_edit(
      &mut text,
      &mut lines,
      Edit {
        pos: 20,
        kind: EditKind::Insert("123\r\n45\r\n6789\r\n"),
      },
    );

    assert_eq!(
      text.as_str(),
      "abcd\r\nefg\r\nhijklm\r\nn123\r\n45\r\n6789\r\no"
    );

    assert_eq!(
      doc_lines(lines),
      vec![
        doc_line(0),
        doc_line(6),
        doc_line(11),
        dirty_doc_line(19),
        dirty_doc_line(25),
        dirty_doc_line(29),
        dirty_doc_line(35),
      ]
    );
  }
}
