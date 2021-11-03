use itertools::Itertools;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use crate::ast::{Program, ProgramLine};
use crate::compiler::compile_prog;
use crate::machine::EmojiStyle;
use crate::machine::MachineProps;
use crate::parser::{parse_line, ParseResult};
use crate::{CodeGen, Diagnostic};

mod binary;

pub struct Document {
  path: PathBuf,
  base_addr: u16,
  emoji_style: EmojiStyle,
  machine_props: Option<MachineProps>,
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
  /// includes EOL
  text: String,
  parsed: Option<T>,
}

#[derive(Debug, Clone)]
pub struct LineDiagnosis {
  pub line_start: usize,
  pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug)]
pub enum DocumentError {
  Io(io::Error),
  UnknownExt(Option<String>),
  LoadBas(binary::LoadError<usize>),
  LoadTxt(binary::LoadError<(usize, usize)>),
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

impl From<io::Error> for DocumentError {
  fn from(err: io::Error) -> Self {
    Self::Io(err)
  }
}

impl From<binary::LoadError<usize>> for DocumentError {
  fn from(err: binary::LoadError<usize>) -> Self {
    Self::LoadBas(err)
  }
}

impl From<binary::LoadError<(usize, usize)>> for DocumentError {
  fn from(err: binary::LoadError<(usize, usize)>) -> Self {
    Self::LoadTxt(err)
  }
}

impl Document {
  /// Load a `.BAS` or `.txt` file.
  pub fn load<P>(path: P) -> Result<Self, DocumentError>
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
          return Err(DocumentError::UnknownExt(ext.map(|ext| ext.to_owned())))
        }
      }
    } else {
      return Err(DocumentError::UnknownExt(None));
    };

    let data = fs::read(path)?;

    let mut doc = if is_bas {
      binary::load_bas(&data, None)?
    } else {
      binary::load_txt(&data, None)?
    };

    let mut emoji_style = doc.guessed_emoji_style;

    let mut machine_props = detect_machine_props(&doc.text)
      .and_then(|p| p.ok())
      .cloned();
    if let Some(props) = &machine_props {
      emoji_style = props.emoji_style;
      doc = if is_bas {
        binary::load_bas(&data, Some(emoji_style))?
      } else {
        binary::load_txt(&data, Some(emoji_style))?
      };
    } else {
      machine_props =
        Some(crate::machine::MACHINES[crate::machine::DEFAULT_MACHINE].clone());
    }

    let lines = text_to_doc_lines(doc.text);

    Ok(Document {
      path: path.to_owned(),
      base_addr: doc.base_addr,
      emoji_style,
      machine_props,
      lines,
      version: DocVer(0),
      compile_cache: None,
    })
  }

  pub fn diagnostics(&mut self) -> Vec<LineDiagnosis> {
    let mut prog = Program {
      lines: self
        .lines
        .iter_mut()
        .map(|line| {
          if let Some(p) = line.parsed.as_ref().cloned() {
            p
          } else {
            let p = parse_line(&line.text).0;
            line.parsed = Some(p.clone());
            p
          }
        })
        .collect(),
    };
    let text = self.text();
    let mut codegen = CodeGen::new(self.emoji_style);
    compile_prog(text, &mut prog, &mut codegen);
    self.compile_cache = Some(CompileCache {
      version: self.version,
      codegen,
    });

    let mut line_diags: Vec<_> = prog
      .lines
      .into_iter()
      .map(|line| LineDiagnosis {
        line_start: 0,
        diagnostics: line.diagnostics,
      })
      .collect();

    let mut line_start = 0;
    for (i, line) in self.lines.iter().enumerate() {
      line_diags[i].line_start = line_start;
      line_start += line.text.len();
    }

    line_diags
  }

  pub fn apply_edit(&mut self, edit: Edit) {
    apply_edit(&mut self.lines, edit);
    self.version.0 += 1;
  }

  pub fn text(&self) -> String {
    self.lines.iter().map(|line| &line.text).join("")
  }
}

fn detect_machine_props(
  text: &str,
) -> Option<Result<&'static MachineProps, ()>> {
  let first_line = text.lines().next().unwrap();
  if let Some(start) = first_line.rfind('{') {
    let first_line = &first_line[start + 1..];
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
      text: text[line_start..line_start + eol + 1].to_owned(),
      parsed: None,
    });
    line_start += eol + 1;
  }
  lines.push(DocLine {
    text: text[line_start..].to_owned(),
    parsed: None,
  });
  lines
}

fn apply_edit(lines: &mut Vec<DocLine>, edit: Edit) {
  let mut offset = 0;
  let mut i = 0;
  while i < lines.len() - 1 && offset + lines[i].text.len() <= edit.pos {
    offset += lines[i].text.len();
    i += 1;
  }

  match edit.kind {
    EditKind::Insert(str) => {
      let start = edit.pos - offset;
      lines[i].text.insert_str(start, &str);
      lines[i].parsed = None;
      if str.contains('\n') {
        let mut new_lines = text_to_doc_lines(&lines[i].text);
        if lines[i].text.ends_with('\n') {
          new_lines.pop();
        }
        lines.splice(i..i + 1, new_lines);
      }
    }
    EditKind::Delete(mut del_len) => {
      let len = lines[i].text.len() + offset - edit.pos;
      lines[i].parsed = None;
      let start = edit.pos - offset;
      let end = (start + del_len).min(lines[i].text.len());
      lines[i].text.replace_range(start..end, "");
      if del_len > len {
        del_len -= len;
        let mut j = i + 1;
        while del_len > 0 {
          lines[j].parsed = None;
          let end = del_len.min(lines[j].text.len());
          lines[j].text.replace_range(..end, "");
          del_len -= end;
          j += 1;
        }
        let i = if lines[i].text.is_empty() { i } else { i + 1 };
        let j = if lines[j - 1].text.is_empty() {
          j
        } else {
          j - 1
        };
        lines.drain(i..j);
      } else if start == 0 && del_len == len {
        lines.remove(i);
      }
      if i < lines.len() - 1 && !lines[i].text.ends_with('\n') {
        let next_line = lines.remove(i + 1).text;
        lines[i].text.push_str(&next_line);
      }
    }
  }

  if lines.last().unwrap().text.ends_with('\n') {
    lines.push(DocLine {
      text: String::new(),
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

  fn doc_line(text: impl ToString) -> DocLine<()> {
    let text = text.to_string();
    DocLine {
      parsed: Some(()),
      text,
    }
  }

  fn dirty_doc_line(text: impl ToString) -> DocLine<()> {
    let text = text.to_string();
    DocLine { parsed: None, text }
  }

  fn doc_lines(lines: Vec<DocLine>) -> Vec<DocLine<()>> {
    lines
      .into_iter()
      .map(|line| DocLine {
        text: line.text,
        parsed: line.parsed.map(|_| ()),
      })
      .collect()
  }

  fn dummy_parsed(source_len: usize) -> ParseResult<ProgramLine> {
    ParseResult {
      stmt_arena: Arena::new(),
      expr_arena: Arena::new(),
      content: ProgramLine {
        source_len,
        label: None,
        stmts: SmallVec::new(),
        eol: Eol::CrLf,
      },
      diagnostics: vec![],
    }
  }

  fn make_lines(text: &str) -> Vec<DocLine> {
    let text = text.replace('\n', "\r\n");
    let mut lines = text_to_doc_lines(text);
    for line in &mut lines {
      line.parsed = Some(dummy_parsed(line.text.len()));
    }
    lines
  }

  const INPUT: &str = "\
abcd
efg
hijklm
no";

  #[test]
  fn delete_middle() {
    let mut lines = make_lines(INPUT);
    apply_edit(
      &mut lines,
      Edit {
        pos: 13,
        kind: EditKind::Delete(3),
      },
    );

    assert_eq!(
      doc_lines(lines),
      vec![
        doc_line("abcd\r\n"),
        doc_line("efg\r\n"),
        dirty_doc_line("him\r\n"),
        doc_line("no"),
      ]
    );
  }

  #[test]
  fn delete_to_end() {
    let mut lines = make_lines(INPUT);
    apply_edit(
      &mut lines,
      Edit {
        pos: 1,
        kind: EditKind::Delete(3),
      },
    );

    assert_eq!(
      doc_lines(lines),
      vec![
        dirty_doc_line("a\r\n"),
        doc_line("efg\r\n"),
        doc_line("hijklm\r\n"),
        doc_line("no"),
      ]
    );
  }

  #[test]
  fn delete_to_newline() {
    let mut lines = make_lines(INPUT);
    apply_edit(
      &mut lines,
      Edit {
        pos: 1,
        kind: EditKind::Delete(5),
      },
    );

    assert_eq!(
      doc_lines(lines),
      vec![
        dirty_doc_line("aefg\r\n"),
        doc_line("hijklm\r\n"),
        doc_line("no"),
      ]
    );
  }

  #[test]
  fn delete_newline() {
    let mut lines = make_lines(INPUT);
    apply_edit(
      &mut lines,
      Edit {
        pos: 4,
        kind: EditKind::Delete(2),
      },
    );

    assert_eq!(
      doc_lines(lines),
      vec![
        dirty_doc_line("abcdefg\r\n"),
        doc_line("hijklm\r\n"),
        doc_line("no"),
      ]
    );
  }

  #[test]
  fn delete_join_lines() {
    let mut lines = make_lines(INPUT);
    apply_edit(
      &mut lines,
      Edit {
        pos: 3,
        kind: EditKind::Delete(4),
      },
    );

    assert_eq!(
      doc_lines(lines),
      vec![
        dirty_doc_line("abcfg\r\n"),
        doc_line("hijklm\r\n"),
        doc_line("no"),
      ]
    );
  }

  #[test]
  fn delete_first_line() {
    let mut lines = make_lines(INPUT);
    apply_edit(
      &mut lines,
      Edit {
        pos: 0,
        kind: EditKind::Delete(6),
      },
    );

    assert_eq!(
      doc_lines(lines),
      vec![doc_line("efg\r\n"), doc_line("hijklm\r\n"), doc_line("no"),]
    );
  }

  #[test]
  fn delete_last_line() {
    let mut lines = make_lines(INPUT);
    apply_edit(
      &mut lines,
      Edit {
        pos: 19,
        kind: EditKind::Delete(2),
      },
    );

    assert_eq!(
      doc_lines(lines),
      vec![
        doc_line("abcd\r\n"),
        doc_line("efg\r\n"),
        doc_line("hijklm\r\n"),
        dirty_doc_line(""),
      ]
    );
  }

  #[test]
  fn delete_middle_line() {
    let mut lines = make_lines(INPUT);
    apply_edit(
      &mut lines,
      Edit {
        pos: 11,
        kind: EditKind::Delete(8),
      },
    );

    assert_eq!(
      doc_lines(lines),
      vec![doc_line("abcd\r\n"), doc_line("efg\r\n"), doc_line("no"),]
    );
  }

  #[test]
  fn delete_first_multiple_lines() {
    let mut lines = make_lines(INPUT);
    apply_edit(
      &mut lines,
      Edit {
        pos: 0,
        kind: EditKind::Delete(19),
      },
    );

    assert_eq!(doc_lines(lines), vec![doc_line("no")]);
  }

  #[test]
  fn delete_last_multiple_lines() {
    let mut lines = make_lines(INPUT);
    apply_edit(
      &mut lines,
      Edit {
        pos: 6,
        kind: EditKind::Delete(15),
      },
    );

    assert_eq!(
      doc_lines(lines),
      vec![doc_line("abcd\r\n"), dirty_doc_line("")]
    );
  }

  #[test]
  fn delete_middle_multiple_lines() {
    let mut lines = make_lines(INPUT);
    apply_edit(
      &mut lines,
      Edit {
        pos: 6,
        kind: EditKind::Delete(13),
      },
    );

    assert_eq!(doc_lines(lines), vec![doc_line("abcd\r\n"), doc_line("no")]);
  }

  #[test]
  fn delete_across_multiple_lines() {
    let mut lines = make_lines(INPUT);
    apply_edit(
      &mut lines,
      Edit {
        pos: 2,
        kind: EditKind::Delete(11),
      },
    );

    assert_eq!(
      doc_lines(lines),
      vec![dirty_doc_line("abjklm\r\n"), doc_line("no")]
    );
  }

  #[test]
  fn delete_across_multiple_lines_until_newline() {
    let mut lines = make_lines(INPUT);
    apply_edit(
      &mut lines,
      Edit {
        pos: 2,
        kind: EditKind::Delete(15),
      },
    );

    assert_eq!(
      doc_lines(lines),
      vec![dirty_doc_line("ab\r\n"), doc_line("no")]
    );
  }

  #[test]
  fn delete_across_multiple_lines_newline() {
    let mut lines = make_lines(INPUT);
    apply_edit(
      &mut lines,
      Edit {
        pos: 2,
        kind: EditKind::Delete(17),
      },
    );

    assert_eq!(doc_lines(lines), vec![dirty_doc_line("abno")]);
  }

  #[test]
  fn insert_middle() {
    let mut lines = make_lines(INPUT);
    apply_edit(
      &mut lines,
      Edit {
        pos: 13,
        kind: EditKind::Insert("123"),
      },
    );

    assert_eq!(
      doc_lines(lines),
      vec![
        doc_line("abcd\r\n"),
        doc_line("efg\r\n"),
        dirty_doc_line("hi123jklm\r\n"),
        doc_line("no")
      ]
    );
  }

  #[test]
  fn insert_after_newline() {
    let mut lines = make_lines(INPUT);
    apply_edit(
      &mut lines,
      Edit {
        pos: 11,
        kind: EditKind::Insert("123"),
      },
    );

    assert_eq!(
      doc_lines(lines),
      vec![
        doc_line("abcd\r\n"),
        doc_line("efg\r\n"),
        dirty_doc_line("123hijklm\r\n"),
        doc_line("no")
      ]
    );
  }

  #[test]
  fn insert_at_start() {
    let mut lines = make_lines(INPUT);
    apply_edit(
      &mut lines,
      Edit {
        pos: 0,
        kind: EditKind::Insert("123"),
      },
    );

    assert_eq!(
      doc_lines(lines),
      vec![
        dirty_doc_line("123abcd\r\n"),
        doc_line("efg\r\n"),
        doc_line("hijklm\r\n"),
        doc_line("no")
      ]
    );
  }

  #[test]
  fn insert_before_newline() {
    let mut lines = make_lines(INPUT);
    apply_edit(
      &mut lines,
      Edit {
        pos: 17,
        kind: EditKind::Insert("123"),
      },
    );

    assert_eq!(
      doc_lines(lines),
      vec![
        doc_line("abcd\r\n"),
        doc_line("efg\r\n"),
        dirty_doc_line("hijklm123\r\n"),
        doc_line("no")
      ]
    );
  }

  #[test]
  fn insert_at_end() {
    let mut lines = make_lines(&INPUT[..INPUT.len() - 2]);
    apply_edit(
      &mut lines,
      Edit {
        pos: 19,
        kind: EditKind::Insert("no"),
      },
    );

    assert_eq!(
      doc_lines(lines),
      vec![
        doc_line("abcd\r\n"),
        doc_line("efg\r\n"),
        doc_line("hijklm\r\n"),
        dirty_doc_line("no")
      ]
    );
  }

  #[test]
  fn insert_multiple_lines_at_start() {
    let mut lines = make_lines(INPUT);
    apply_edit(
      &mut lines,
      Edit {
        pos: 0,
        kind: EditKind::Insert("123\r\n45\r\n"),
      },
    );

    assert_eq!(
      doc_lines(lines),
      vec![
        dirty_doc_line("123\r\n"),
        dirty_doc_line("45\r\n"),
        dirty_doc_line("abcd\r\n"),
        doc_line("efg\r\n"),
        doc_line("hijklm\r\n"),
        doc_line("no")
      ]
    );
  }

  #[test]
  fn insert_multiple_lines() {
    let mut lines = make_lines(INPUT);
    apply_edit(
      &mut lines,
      Edit {
        pos: 13,
        kind: EditKind::Insert("123\r\n45\r\n6789"),
      },
    );

    assert_eq!(
      doc_lines(lines),
      vec![
        doc_line("abcd\r\n"),
        doc_line("efg\r\n"),
        dirty_doc_line("hi123\r\n"),
        dirty_doc_line("45\r\n"),
        dirty_doc_line("6789jklm\r\n"),
        doc_line("no")
      ]
    );
  }

  #[test]
  fn insert_multiple_lines_at_end() {
    let mut lines = make_lines(INPUT);
    apply_edit(
      &mut lines,
      Edit {
        pos: 21,
        kind: EditKind::Insert("123\r\n45\r\n6789"),
      },
    );

    assert_eq!(
      doc_lines(lines),
      vec![
        doc_line("abcd\r\n"),
        doc_line("efg\r\n"),
        doc_line("hijklm\r\n"),
        dirty_doc_line("no123\r\n"),
        dirty_doc_line("45\r\n"),
        dirty_doc_line("6789"),
      ]
    );
  }

  #[test]
  fn insert_multiple_lines_and_newline_at_end() {
    let mut lines = make_lines(INPUT);
    apply_edit(
      &mut lines,
      Edit {
        pos: 21,
        kind: EditKind::Insert("123\r\n45\r\n6789\r\n"),
      },
    );

    assert_eq!(
      doc_lines(lines),
      vec![
        doc_line("abcd\r\n"),
        doc_line("efg\r\n"),
        doc_line("hijklm\r\n"),
        dirty_doc_line("no123\r\n"),
        dirty_doc_line("45\r\n"),
        dirty_doc_line("6789\r\n"),
        dirty_doc_line(""),
      ]
    );
  }
}
