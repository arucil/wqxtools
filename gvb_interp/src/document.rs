use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::ast::{Eol, Label, Program, ProgramLine};
use crate::compiler::compile_prog;
use crate::device::default::DefaultDevice;
use crate::device::Device;
use crate::machine::EmojiVersion;
use crate::machine::MachineProps;
use crate::parser::{parse_line, ParseResult};
use crate::{CodeGen, Diagnostic, VirtualMachine};

mod binary;

const DEFAULT_TEXT: &str = "10 ";

pub struct Document {
  base_addr: u16,
  emoji_version: EmojiVersion,
  machine_props: MachineProps,
  text: String,
  lines: Vec<DocLine>,
  version: DocVer,
  compile_cache: Option<CompileCache>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DocVer(u64);

struct CompileCache {
  diagnostics: Vec<LineDiagnosis>,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplaceText {
  pub pos: usize,
  pub old_len: usize,
  pub str: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelTarget {
  PrevLine,
  CurLine,
  NextLine,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddLabelResult {
  pub edit: ReplaceText,
  pub goto: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AddLabelError {
  AlreadyHasLabel,
  CannotInferLabel,
}

#[derive(Debug)]
pub struct ReplaceChar {
  pub pos: usize,
  pub old_len: usize,
  pub ch: char,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MachinePropError {
  NotFound(String),
  Save(binary::SaveError),
  Load(binary::LoadError<(usize, usize)>),
}

impl From<binary::SaveError> for MachinePropError {
  fn from(err: binary::SaveError) -> Self {
    Self::Save(err)
  }
}

impl From<binary::LoadError<(usize, usize)>> for MachinePropError {
  fn from(err: binary::LoadError<(usize, usize)>) -> Self {
    Self::Load(err)
  }
}

impl Document {
  pub fn new() -> Self {
    Self {
      base_addr: binary::DEFAULT_BASE_ADDR,
      emoji_version: EmojiVersion::New,
      machine_props: crate::machine::machines()
        [EmojiVersion::New.default_machine_name()]
      .clone(),
      text: DEFAULT_TEXT.to_owned(),
      lines: text_to_doc_lines(DEFAULT_TEXT),
      version: DocVer(0),
      compile_cache: None,
    }
  }

  pub fn load<D>(data: D, is_bas: bool) -> Result<Self, LoadDocumentError>
  where
    D: AsRef<[u8]>,
  {
    let mut doc = if is_bas {
      binary::load_bas(&data, None)?
    } else {
      binary::load_txt(&data, None)?
    };

    let mut emoji_version = doc.guessed_emoji_version;

    let machine_props;
    if let Some(props) = detect_machine_props(&doc.text).and_then(|p| p.1.ok())
    {
      emoji_version = props.emoji_version;
      doc = if is_bas {
        binary::load_bas(&data, Some(emoji_version))?
      } else {
        binary::load_txt(&data, Some(emoji_version))?
      };
      machine_props = props;
    } else {
      machine_props = crate::machine::machines()
        [emoji_version.default_machine_name()]
      .clone();
    }

    let lines = text_to_doc_lines(&doc.text);

    Ok(Document {
      base_addr: doc.base_addr,
      emoji_version,
      machine_props,
      text: doc.text,
      lines,
      version: DocVer(0),
      compile_cache: None,
    })
  }

  /// Load a `.bas` or `.txt` file.
  pub fn load_file<P>(path: P) -> Result<Self, LoadDocumentError>
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
    Self::load(data, is_bas)
  }

  /// Save to a `.bas` or `.txt` file.
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
      binary::save_bas(&self.text, self.emoji_version, self.base_addr)?
    } else {
      binary::save_txt(&self.text, self.emoji_version)?
    };

    fs::write(path, data)?;

    Ok(())
  }

  pub fn diagnostics(&mut self) -> &[LineDiagnosis] {
    if let Some(cache) = &self.compile_cache {
      if cache.version == self.version {
        let len = cache.diagnostics.len();
        let ptr = cache.diagnostics.as_ptr();
        // TODO remove unsafe when Polonius borrow checker is done
        return unsafe { std::slice::from_raw_parts(ptr, len) };
      }
    }

    let mut prog = Program {
      lines: Vec::with_capacity(self.lines.len()),
    };
    for i in 0..self.lines.len() {
      prog.lines.push(self.ensure_line_parsed(i).clone());
    }
    let mut codegen = CodeGen::new(self.emoji_version);
    compile_prog(&self.text, &mut prog, &mut codegen);

    let diagnostics = prog
      .lines
      .into_iter()
      .zip(&self.lines)
      .map(|(line, doc_line)| LineDiagnosis {
        line_start: doc_line.line_start,
        diagnostics: line.diagnostics,
      })
      .collect();

    self.compile_cache = Some(CompileCache {
      diagnostics,
      version: self.version,
      codegen,
    });

    &self.compile_cache.as_ref().unwrap().diagnostics
  }

  fn ensure_line_parsed(&mut self, i: usize) -> &ParseResult<ProgramLine> {
    if let Some(p) = self.lines[i].parsed.as_ref() {
      // TODO remove unsafe after Polonius is done
      return unsafe { &*(p as *const _) };
    }
    let start = self.lines[i].line_start;
    let end = self
      .lines
      .get(i + 1)
      .map_or(self.text.len(), |line| line.line_start);
    let p = parse_line(&self.text[start..end]).0;
    self.lines[i].parsed = Some(p.clone());
    self.lines[i].parsed.as_ref().unwrap()
  }

  pub fn apply_edit(&mut self, edit: Edit) {
    apply_edit(&mut self.text, &mut self.lines, edit);
    self.version.0 += 1;
  }

  pub fn text(&self) -> &str {
    &self.text
  }

  pub fn machine_name(&self) -> &str {
    &self.machine_props.name
  }

  pub fn sync_machine_name(
    &mut self,
  ) -> Result<Vec<ReplaceChar>, MachinePropError> {
    let props = match detect_machine_props(&self.text) {
      Some((_, Ok(props))) => props,
      Some((_, Err(name))) => {
        return Err(MachinePropError::NotFound(name));
      }
      None => crate::machine::machines()
        [self.emoji_version.default_machine_name()]
      .clone(),
    };

    let saved = binary::save_txt(&self.text, self.emoji_version)?;
    let text = binary::load_txt(saved, Some(props.emoji_version))?.text;

    let mut edits = vec![];
    for ((i, c1), c2) in self.text.char_indices().zip(text.chars()) {
      if c1 != c2 {
        edits.push(ReplaceChar {
          pos: i,
          old_len: c1.len_utf8(),
          ch: c2,
        });
      }
    }

    self.emoji_version = props.emoji_version;
    self.machine_props = props;
    self.lines = text_to_doc_lines(&text);
    self.text = text;

    Ok(edits)
  }

  pub fn compute_machine_name_edit(
    &self,
    name: &str,
  ) -> Result<ReplaceText, MachinePropError> {
    let name = name.to_ascii_uppercase();
    if let Some(props) = crate::machine::machines().get(&name) {
      let saved = binary::save_txt(&self.text, self.emoji_version)?;
      binary::load_txt(saved, Some(props.emoji_version))?;
    } else {
      return Err(MachinePropError::NotFound(name));
    }
    match detect_machine_props(&self.text) {
      Some(((start, end), _)) => Ok(ReplaceText {
        pos: start,
        old_len: end - start,
        str: name,
      }),
      None => {
        use std::fmt::Write;

        let first_line = self.text.lines().next().unwrap();
        let quotes =
          first_line.as_bytes().iter().filter(|&&c| c == b'"').count();
        let mut str = String::new();
        if quotes % 2 != 0 {
          str.push_str("\":");
        } else if self.text.as_bytes()[first_line.len() - 1] != b':' {
          str.push(':');
        }
        str.push_str("REM {type:");
        write!(&mut str, "{}", name).unwrap();
        str.push('}');
        Ok(ReplaceText {
          pos: first_line.len(),
          old_len: 0,
          str,
        })
      }
    }
  }

  pub fn compute_add_label_edit(
    &mut self,
    target: LabelTarget,
    cursor_pos: usize,
  ) -> Result<AddLabelResult, AddLabelError> {
    fn infer_label(
      lb: Option<u16>,
      ub: Option<u16>,
      lower: bool,
    ) -> Option<u16> {
      match (lb, ub) {
        (None, Some(ub)) => {
          if ub >= 10 {
            let r = ub % 10;
            if r == 0 {
              Some(ub - 10)
            } else {
              Some(ub - r)
            }
          } else if ub > 0 {
            Some(ub - 1)
          } else {
            None
          }
        }
        (Some(lb), None) => {
          if lb <= 9989 {
            Some(lb + 10 - lb % 10)
          } else if lb < 9999 {
            Some(lb + 1)
          } else {
            None
          }
        }
        (Some(lb), Some(ub)) => {
          if ub - lb > 10 {
            if lower {
              Some(lb + 10 - lb % 10)
            } else {
              let r = ub % 10;
              if r == 0 {
                Some(ub - 10)
              } else {
                Some(ub - r)
              }
            }
          } else if ub - lb > 1 {
            if lower {
              Some(lb + 1)
            } else {
              Some(ub - 1)
            }
          } else {
            None
          }
        }
        (None, None) => None,
      }
    }

    let i = find_line_by_position(&self.lines, cursor_pos);
    match target {
      LabelTarget::CurLine => {
        if self.ensure_line_parsed(i).content.label.is_some() {
          return Err(AddLabelError::AlreadyHasLabel);
        }
        let lb = if i > 0 {
          Some(self.line_label(i - 1)?)
        } else {
          None
        };
        let ub = if i < self.lines.len() - 1 {
          Some(self.line_label(i + 1)?)
        } else {
          None
        };
        let label = if self.lines.len() == 1 {
          10
        } else {
          infer_label(lb, ub, true).ok_or(AddLabelError::CannotInferLabel)?
        };
        let pos = self.lines[i].line_start;
        let goto;
        let str;
        if self.text.len() > pos && self.text.as_bytes()[pos] == b' ' {
          goto = None;
          str = label.to_string();
        } else {
          str = format!("{} ", label);
          if pos == cursor_pos {
            goto = Some(pos + str.len());
          } else {
            goto = None;
          }
        }
        Ok(AddLabelResult {
          edit: ReplaceText {
            pos,
            old_len: 0,
            str,
          },
          goto,
        })
      }
      LabelTarget::PrevLine => {
        let lb = if i > 0 {
          Some(self.line_label(i - 1)?)
        } else {
          None
        };
        let ub = Some(self.line_label(i)?);
        let label =
          infer_label(lb, ub, false).ok_or(AddLabelError::CannotInferLabel)?;
        let pos = self.lines[i].line_start;
        let str = format!("{} {}", label, Eol::CrLf);
        let goto = pos + str.len() - Eol::CrLf.byte_len();
        Ok(AddLabelResult {
          goto: Some(goto),
          edit: ReplaceText {
            pos,
            old_len: 0,
            str,
          },
        })
      }
      LabelTarget::NextLine => {
        let lb = Some(self.line_label(i)?);
        let ub = if i < self.lines.len() - 1 {
          Some(self.line_label(i + 1)?)
        } else {
          None
        };
        let label =
          infer_label(lb, ub, true).ok_or(AddLabelError::CannotInferLabel)?;
        let str;
        let pos;
        let goto;
        if i == self.lines.len() - 1 {
          str = format!("{}{} ", Eol::CrLf, label);
          pos = self.text.len();
          goto = pos + str.len();
        } else {
          str = format!("{} {}", label, Eol::CrLf);
          pos = self.lines[i + 1].line_start;
          goto = pos + str.len() - Eol::CrLf.byte_len();
        }
        Ok(AddLabelResult {
          goto: Some(goto),
          edit: ReplaceText {
            pos,
            old_len: 0,
            str,
          },
        })
      }
    }
  }

  fn line_label(&mut self, i: usize) -> Result<u16, AddLabelError> {
    self
      .ensure_line_parsed(i)
      .content
      .label
      .as_ref()
      .map(|(_, Label(label))| *label)
      .ok_or(AddLabelError::CannotInferLabel)
  }

  pub fn create_device<P>(&self, data_dir: P) -> DefaultDevice
  where
    P: Into<PathBuf>,
  {
    DefaultDevice::new(self.machine_props.clone(), data_dir)
  }

  /// If the document contains errors, Err is returned.
  pub fn create_vm<'d, D>(
    &mut self,
    device: &'d mut D,
  ) -> Result<VirtualMachine<'d, D>, ()>
  where
    D: Device,
  {
    let diagnostics = self.diagnostics();
    if diagnostics.iter().any(|d| d.contains_errors()) {
      return Err(());
    }

    let codegen = self.compile_cache.as_ref().unwrap().codegen.clone();
    Ok(VirtualMachine::new(codegen, device))
  }
}

impl LineDiagnosis {
  fn contains_errors(&self) -> bool {
    crate::contains_errors(&self.diagnostics)
  }
}

fn detect_machine_props(
  text: impl AsRef<str>,
) -> Option<((usize, usize), Result<MachineProps, String>)> {
  let first_line = text.as_ref().lines().next().unwrap();
  if let Some(start) = first_line.rfind("{type:") {
    let start = start + "{type:".len();
    let first_line = &first_line[start..];
    if let Some(end) = first_line.find('}') {
      let name = first_line[..end].trim().to_ascii_uppercase();
      if !name.is_empty() {
        match crate::machine::machines().get(&name) {
          Some(props) => {
            return Some(((start, start + end), Ok(props.clone())))
          }
          None => return Some(((start, start + end), Err(name))),
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

fn find_line_by_position(lines: &[DocLine], pos: usize) -> usize {
  let mut lo = 0;
  let mut hi = lines.len();
  while lo < hi {
    let mid = (lo + hi) / 2;
    if pos >= lines[mid].line_start {
      lo = mid + 1;
    } else {
      hi = mid;
    }
  }
  lo - 1
}

fn apply_edit(text: &mut String, lines: &mut Vec<DocLine>, edit: Edit) {
  let mut i = find_line_by_position(lines, edit.pos);

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

  fn make_doc(text: &str) -> Document {
    let text = text.replace('\n', "\r\n");
    Document::load(text, false).unwrap()
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
        kind: EditKind::Insert("123".into()),
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
        kind: EditKind::Insert("123".into()),
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
        kind: EditKind::Insert("123".into()),
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
        kind: EditKind::Insert("123".into()),
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
        kind: EditKind::Insert("123".into()),
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
        kind: EditKind::Insert("no".into()),
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
        kind: EditKind::Insert("123\r\n45\r\n".into()),
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
        kind: EditKind::Insert("abcd\r\nefg\r\nhijklm\r\nno".into()),
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
        kind: EditKind::Insert("abcd\r\nefg\r\nhijklm\r\nno\r\n".into()),
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
        kind: EditKind::Insert("123\r\n45\r\n6789".into()),
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
        kind: EditKind::Insert("123\r\n45\r\n6789".into()),
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
        kind: EditKind::Insert("123\r\n45\r\n6789\r\n".into()),
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
        kind: EditKind::Insert("123\r\n45\r\n6789".into()),
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
        kind: EditKind::Insert("123\r\n45\r\n6789\r\n".into()),
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
        kind: EditKind::Insert("123\r\n45\r\n6789".into()),
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
        kind: EditKind::Insert("123\r\n45\r\n6789\r\n".into()),
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
        kind: EditKind::Insert("123\r\n45\r\n6789".into()),
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
        kind: EditKind::Insert("123\r\n45\r\n6789\r\n".into()),
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

  #[test]
  fn add_machine_name() {
    let doc = make_doc(
      r#"
10 cls
20 :::
"#
      .trim(),
    );
    assert_eq!(
      doc.compute_machine_name_edit("PC1000A"),
      Ok(ReplaceText {
        pos: 6,
        old_len: 0,
        str: ":REM {type:PC1000A}".to_owned(),
      })
    );
  }

  #[test]
  fn add_machine_name_quote() {
    let doc = make_doc(
      r#"
10 cls:print "foo
20 :::
"#
      .trim(),
    );
    assert_eq!(
      doc.compute_machine_name_edit("pc1000a"),
      Ok(ReplaceText {
        pos: 17,
        old_len: 0,
        str: "\":REM {type:PC1000A}".to_owned(),
      })
    );
  }

  #[test]
  fn modify_machine_name() {
    let doc = make_doc(
      r#"
10 cls:rem {type:pc1000a}
20 :::
"#
      .trim(),
    );
    assert_eq!(
      doc.compute_machine_name_edit("tc808"),
      Ok(ReplaceText {
        pos: 17,
        old_len: 7,
        str: "TC808".to_owned(),
      })
    );
  }

  mod add_label {
    use super::*;
    use pretty_assertions::assert_eq;

    #[test]
    fn cur_line_has_label() {
      let mut doc = make_doc(
        r#"
10 cls
:::
23 text
"#
        .trim(),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::CurLine, 4),
        Err(AddLabelError::AlreadyHasLabel)
      );
    }

    #[test]
    fn cur_line_has_space() {
      let mut doc = make_doc(
        r#"
10 cls
 :::
23 text
"#
        .trim(),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::CurLine, 9),
        Ok(AddLabelResult {
          edit: ReplaceText {
            pos: 8,
            old_len: 0,
            str: format!("20"),
          },
          goto: None
        })
      );
    }

    #[test]
    fn lb_ub_10() {
      let mut doc = make_doc(
        r#"
09 cls
:::
23 text
34 graph
"#
        .trim(),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::CurLine, 9),
        Ok(AddLabelResult {
          edit: ReplaceText {
            pos: 8,
            old_len: 0,
            str: format!("10 "),
          },
          goto: None
        })
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::PrevLine, 22),
        Ok(AddLabelResult {
          edit: ReplaceText {
            pos: 22,
            old_len: 0,
            str: format!("30 \r\n"),
          },
          goto: Some(25)
        })
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::NextLine, 14),
        Ok(AddLabelResult {
          edit: ReplaceText {
            pos: 22,
            old_len: 0,
            str: format!("30 \r\n"),
          },
          goto: Some(25)
        })
      );
    }

    #[test]
    fn lb_ub_1() {
      let mut doc = make_doc(
        r#"
10 cls
:::
16 text
26 graph
"#
        .trim(),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::CurLine, 9),
        Ok(AddLabelResult {
          edit: ReplaceText {
            pos: 8,
            old_len: 0,
            str: format!("11 "),
          },
          goto: None
        })
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::PrevLine, 22),
        Ok(AddLabelResult {
          edit: ReplaceText {
            pos: 22,
            old_len: 0,
            str: format!("25 \r\n"),
          },
          goto: Some(25)
        })
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::NextLine, 14),
        Ok(AddLabelResult {
          edit: ReplaceText {
            pos: 22,
            old_len: 0,
            str: format!("17 \r\n"),
          },
          goto: Some(25)
        })
      );
    }

    #[test]
    fn lb_ub_0() {
      let mut doc = make_doc(
        r#"
10 cls
:::
11 text
12 graph
"#
        .trim(),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::CurLine, 9),
        Err(AddLabelError::CannotInferLabel),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::PrevLine, 22),
        Err(AddLabelError::CannotInferLabel),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::NextLine, 14),
        Err(AddLabelError::CannotInferLabel),
      );
    }

    #[test]
    fn cur_line_one_line() {
      let mut doc = make_doc(
        r#"
:::
"#
        .trim(),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::CurLine, 2),
        Ok(AddLabelResult {
          edit: ReplaceText {
            pos: 0,
            old_len: 0,
            str: format!("10 "),
          },
          goto: None
        })
      );
    }

    #[test]
    fn lb_no_ub() {
      let mut doc = make_doc(
        r#"
10 :::
:::
:::
30 graph
:::
"#
        .trim(),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::CurLine, 9),
        Err(AddLabelError::CannotInferLabel),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::PrevLine, 9),
        Err(AddLabelError::CannotInferLabel),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::NextLine, 19),
        Err(AddLabelError::CannotInferLabel),
      );
    }

    #[test]
    fn ub_no_lb() {
      let mut doc = make_doc(
        r#"
:::
:::
10 :::
"#
        .trim(),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::CurLine, 5),
        Err(AddLabelError::CannotInferLabel),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::PrevLine, 10),
        Err(AddLabelError::CannotInferLabel),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::NextLine, 5),
        Err(AddLabelError::CannotInferLabel),
      );
    }

    #[test]
    fn no_lb_no_ub() {
      let mut doc = make_doc(
        r#"
:::
:::
:::
"#
        .trim(),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::CurLine, 6),
        Err(AddLabelError::CannotInferLabel),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::PrevLine, 6),
        Err(AddLabelError::CannotInferLabel),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::NextLine, 6),
        Err(AddLabelError::CannotInferLabel),
      );
    }

    #[test]
    fn first_line_10() {
      let mut doc = make_doc(
        r#"
:::
15 cls
"#
        .trim(),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::CurLine, 1),
        Ok(AddLabelResult {
          edit: ReplaceText {
            pos: 0,
            old_len: 0,
            str: format!("10 "),
          },
          goto: None
        })
      );

      let mut doc = make_doc(
        r#"
15 cls
"#
        .trim(),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::PrevLine, 0),
        Ok(AddLabelResult {
          edit: ReplaceText {
            pos: 0,
            old_len: 0,
            str: format!("10 \r\n"),
          },
          goto: Some(3)
        })
      );
    }

    #[test]
    fn first_line_1() {
      let mut doc = make_doc(
        r#"
:::
5 cls
"#
        .trim(),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::CurLine, 1),
        Ok(AddLabelResult {
          edit: ReplaceText {
            pos: 0,
            old_len: 0,
            str: format!("4 "),
          },
          goto: None
        })
      );

      let mut doc = make_doc(
        r#"
5 cls
"#
        .trim(),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::PrevLine, 1),
        Ok(AddLabelResult {
          edit: ReplaceText {
            pos: 0,
            old_len: 0,
            str: format!("4 \r\n"),
          },
          goto: Some(2)
        })
      );
    }

    #[test]
    fn first_line_0() {
      let mut doc = make_doc(
        r#"
:::
0 cls
"#
        .trim(),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::CurLine, 1),
        Err(AddLabelError::CannotInferLabel),
      );

      let mut doc = make_doc(
        r#"
0 cls
"#
        .trim(),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::PrevLine, 1),
        Err(AddLabelError::CannotInferLabel),
      );
    }

    #[test]
    fn first_line_no_ub() {
      let mut doc = make_doc(
        r#"
:::
cls
"#
        .trim(),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::CurLine, 1),
        Err(AddLabelError::CannotInferLabel),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::PrevLine, 1),
        Err(AddLabelError::CannotInferLabel),
      );
    }

    #[test]
    fn last_line_10() {
      let mut doc = make_doc(
        r#"
9989 cls
:::
"#
        .trim(),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::CurLine, 12),
        Ok(AddLabelResult {
          edit: ReplaceText {
            pos: 10,
            old_len: 0,
            str: format!("9990 "),
          },
          goto: None
        })
      );

      let mut doc = make_doc(
        r#"
9989 cls
"#
        .trim(),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::NextLine, 0),
        Ok(AddLabelResult {
          edit: ReplaceText {
            pos: 8,
            old_len: 0,
            str: format!("\r\n9990 "),
          },
          goto: Some(15)
        })
      );
    }

    #[test]
    fn last_line_1() {
      let mut doc = make_doc(
        r#"
9993 cls
:::
"#
        .trim(),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::CurLine, 12),
        Ok(AddLabelResult {
          edit: ReplaceText {
            pos: 10,
            old_len: 0,
            str: format!("9994 "),
          },
          goto: None
        })
      );

      let mut doc = make_doc(
        r#"
9993 cls
"#
        .trim(),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::NextLine, 8),
        Ok(AddLabelResult {
          edit: ReplaceText {
            pos: 8,
            old_len: 0,
            str: format!("\r\n9994 "),
          },
          goto: Some(15)
        })
      );
    }

    #[test]
    fn last_line_0() {
      let mut doc = make_doc(
        r#"
9999 cls
:::
"#
        .trim(),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::CurLine, 12),
        Err(AddLabelError::CannotInferLabel),
      );

      let mut doc = make_doc(
        r#"
9999 cls
"#
        .trim(),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::NextLine, 0),
        Err(AddLabelError::CannotInferLabel),
      );
    }

    #[test]
    fn last_line_no_lb() {
      let mut doc = make_doc(
        r#"
cls
:::
"#
        .trim(),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::CurLine, 5),
        Err(AddLabelError::CannotInferLabel),
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::NextLine, 5),
        Err(AddLabelError::CannotInferLabel),
      );
    }

    #[test]
    fn last_line_empty() {
      let mut doc = make_doc(
        r#"10 cls

20 cls
cls
50 cls
"#,
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::CurLine, 8),
        Ok(AddLabelResult {
          edit: ReplaceText {
            pos: 8,
            old_len: 0,
            str: format!("11 "),
          },
          goto: Some(11)
        })
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::CurLine, 18),
        Ok(AddLabelResult {
          edit: ReplaceText {
            pos: 18,
            old_len: 0,
            str: format!("30 "),
          },
          goto: Some(21)
        })
      );
      assert_eq!(
        doc.compute_add_label_edit(LabelTarget::CurLine, 31),
        Ok(AddLabelResult {
          edit: ReplaceText {
            pos: 31,
            old_len: 0,
            str: format!("60 "),
          },
          goto: Some(34)
        })
      );
    }
  }
}
