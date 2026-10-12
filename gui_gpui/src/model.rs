use gpui_kit::base::input::{Diagnostic, DiagnosticSeverity, Position};
use gvb_interp::{
  Document, Edit, EditKind, MachinePropError, ReplaceText, Severity,
};
use widestring::Utf16String;

pub struct EditorModel {
  document: Document,
  source: String,
}

pub struct Analysis {
  pub diagnostics: Vec<Diagnostic>,
  pub problems: Vec<Problem>,
  pub errors: usize,
  pub warnings: usize,
  pub first_problem: Option<String>,
}

#[derive(Clone)]
pub struct Problem {
  pub line: usize,
  pub column: usize,
  pub severity: Severity,
  pub message: String,
}

impl EditorModel {
  pub fn new() -> Self {
    let document = Document::new();
    let source = document.text().to_string();
    Self { document, source }
  }

  pub fn document(&self) -> &Document {
    &self.document
  }

  pub fn document_mut(&mut self) -> &mut Document {
    &mut self.document
  }

  pub fn source(&self) -> &str {
    &self.source
  }

  pub fn replace_document(&mut self, document: Document) -> Analysis {
    self.source = document.text().to_string();
    self.document = document;
    self.analyze(&self.source.clone())
  }

  pub fn refresh_from_document(&mut self) -> Analysis {
    self.source = self.document.text().to_string();
    self.analyze(&self.source.clone())
  }

  pub fn apply_replacements(&mut self, edits: &[ReplaceText]) -> Analysis {
    let mut source = Utf16String::from(self.source.as_str());
    for edit in edits {
      let mut next = Utf16String::with_capacity(
        source.len() - edit.range.len() + edit.str.len(),
      );
      next.push_utfstr(&source[..edit.range.start]);
      next.push_utfstr(&edit.str);
      next.push_utfstr(&source[edit.range.end..]);
      source = next;
    }
    self.analyze(&source.to_string())
  }

  pub fn set_machine_name(
    &mut self,
    name: &str,
  ) -> Result<Analysis, MachinePropError> {
    let edit = self
      .document
      .compute_machine_name_edit(&Utf16String::from(name))?;
    let old_source = self.source.clone();
    self.apply_replacements(&[edit]);
    if let Err(error) = self.document.sync_machine_name() {
      self.analyze(&old_source);
      return Err(error);
    }
    Ok(self.refresh_from_document())
  }

  pub fn analyze(&mut self, source: &str) -> Analysis {
    if self.source != source {
      let old_chars: Vec<char> = self.source.chars().collect();
      let new_chars: Vec<char> = source.chars().collect();
      let prefix = old_chars
        .iter()
        .zip(&new_chars)
        .take_while(|(old, new)| old == new)
        .count();
      let suffix = old_chars[prefix..]
        .iter()
        .rev()
        .zip(new_chars[prefix..].iter().rev())
        .take_while(|(old, new)| old == new)
        .count();
      let pos: usize =
        old_chars[..prefix].iter().map(|ch| ch.len_utf16()).sum();
      let delete_len: usize = old_chars[prefix..old_chars.len() - suffix]
        .iter()
        .map(|ch| ch.len_utf16())
        .sum();
      if delete_len != 0 {
        self.document.apply_edit(Edit {
          pos,
          kind: EditKind::Delete(delete_len),
        });
      }
      let insertion: String =
        new_chars[prefix..new_chars.len() - suffix].iter().collect();
      let utf16 = Utf16String::from(insertion);
      if !utf16.is_empty() {
        self.document.apply_edit(Edit {
          pos,
          kind: EditKind::Insert(&utf16),
        });
      }
      self.source = source.to_owned();
    }

    let mut analysis = Analysis {
      diagnostics: Vec::new(),
      problems: Vec::new(),
      errors: 0,
      warnings: 0,
      first_problem: None,
    };
    let source_lines: Vec<_> = self.source.split('\n').collect();
    for (line_number, line) in self.document.diagnostics().iter().enumerate() {
      let line_text = source_lines
        .get(line_number)
        .expect("document diagnostics must match source lines");
      for diagnostic in &line.diagnostics {
        analysis.problems.push(Problem {
          line: line_number + 1,
          column: utf16_to_char_index(line_text, diagnostic.range.start) + 1,
          severity: diagnostic.severity,
          message: diagnostic.message.clone(),
        });
        let severity = match diagnostic.severity {
          Severity::Error => {
            analysis.errors += 1;
            DiagnosticSeverity::Error
          }
          Severity::Warning => {
            analysis.warnings += 1;
            DiagnosticSeverity::Warning
          }
        };
        if analysis.first_problem.is_none() {
          analysis.first_problem = Some(format!(
            "{}:{}  {}",
            line_number + 1,
            diagnostic.range.start + 1,
            diagnostic.message
          ));
        }
        analysis.diagnostics.push(
          Diagnostic::new(
            Position::new(
              line_number as u32,
              utf16_to_char_index(line_text, diagnostic.range.start) as u32,
            )
              ..Position::new(
                line_number as u32,
                utf16_to_char_index(line_text, diagnostic.range.end) as u32,
              ),
            diagnostic.message.clone(),
          )
          .with_severity(severity),
        );
      }
    }
    analysis.diagnostics.sort_by_key(|diagnostic| {
      (
        diagnostic.range.start.line,
        diagnostic.range.start.character,
        diagnostic.range.end.character,
      )
    });
    analysis
  }
}

fn utf16_to_char_index(text: &str, offset: usize) -> usize {
  let mut units = 0;
  for (index, character) in text.chars().enumerate() {
    if units + character.len_utf16() > offset {
      return index;
    }
    units += character.len_utf16();
  }
  text.chars().count()
}

#[cfg(test)]
mod tests {
  use super::*;
  use gpui_kit::base::input::{DiagnosticSet, Rope};

  #[test]
  fn chinese_text_and_real_compiler_diagnostics() {
    gvb_interp::machine::init_machines_from_str(include_str!(
      "../../gvb_interp/machines.yaml"
    ))
    .expect("bundled machines must be valid");
    let mut model = EditorModel::new();
    let valid = model.analyze("10 PRINT \"你好，文曲星\"\n20 END");
    assert_eq!(valid.errors, 0);
    let invalid_source = "10 PRINT \"你好，文曲星\"\n20 @";
    let invalid = model.analyze(invalid_source);
    assert!(invalid.errors > 0);
    assert!(
      invalid
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
    );
    assert!(invalid.diagnostics.iter().any(|diagnostic| {
      diagnostic.range.start.line == 1
        && diagnostic.range.start.character < diagnostic.range.end.character
    }));
    let rope = Rope::from(invalid_source);
    let mut set = DiagnosticSet::new(&rope);
    set.extend(invalid.diagnostics.clone());
    assert_eq!(set.range(0..rope.len()).count(), invalid.diagnostics.len());
    assert_eq!(set.range(30..rope.len()).count(), invalid.diagnostics.len());
    assert!(invalid.diagnostics.windows(2).all(|pair| {
      pair[0].range.start.line < pair[1].range.start.line
        || pair[0].range.start.character <= pair[1].range.start.character
    }));
    assert_eq!(model.analyze("10 PRINT \"你好，文曲星\"\n20 END").errors, 0);
    let warning = model.analyze("10 RESTORE 999\n20 END");
    assert_eq!(warning.errors, 0);
    assert!(warning.warnings > 0);
  }

  #[test]
  fn refresh_from_document_does_not_apply_the_same_edit_twice() {
    gvb_interp::machine::init_machines_from_str(include_str!(
      "../../gvb_interp/machines.yaml"
    ))
    .expect("bundled machines must be valid");
    let mut model = EditorModel::new();
    model.analyze("10 END");
    let prefix = Utf16String::from("5 PRINT 1\n");
    model.document_mut().apply_edit(Edit {
      pos: 0,
      kind: EditKind::Insert(&prefix),
    });
    let expected = model.document().text().to_string();

    model.refresh_from_document();

    assert_eq!(model.source(), expected);
    assert_eq!(model.document().text().to_string(), expected);
  }

  #[test]
  fn utf16_diagnostic_columns_map_to_gpui_scalar_columns() {
    assert_eq!(utf16_to_char_index("你😀@", 0), 0);
    assert_eq!(utf16_to_char_index("你😀@", 1), 1);
    assert_eq!(utf16_to_char_index("你😀@", 3), 2);
    assert_eq!(utf16_to_char_index("你😀@", 4), 3);
  }

  #[test]
  fn incremental_edit_keeps_surrogate_pairs_intact() {
    gvb_interp::machine::init_machines_from_str(include_str!(
      "../../gvb_interp/machines.yaml"
    ))
    .expect("bundled machines must be valid");
    let mut model = EditorModel::new();
    for source in [
      "10 PRINT \"你😀\"\n20 END",
      "10 PRINT \"你🧠\"\n20 END",
      "10 PRINT \"你🧠好\"\n20 END",
      "10 PRINT \"好\"\n20 END",
    ] {
      model.analyze(source);
      assert_eq!(model.document().text().to_string(), source);
    }
  }

  #[test]
  fn machine_selection_updates_source_and_runtime_document() {
    gvb_interp::machine::init_machines_from_str(include_str!(
      "../../gvb_interp/machines.yaml"
    ))
    .expect("bundled machines must be valid");
    let mut model = EditorModel::new();
    model.analyze("10 PRINT \"hello\"\n20 END");
    for name in ["PC1000A", "NC3000", "TC808"] {
      let analysis = model.set_machine_name(name).expect("supported machine");
      assert_eq!(analysis.errors, 0);
      assert_eq!(model.document().machine_name(), name);
      assert!(model.source().contains(&format!("{{type:{name}}}")));
      assert_eq!(model.source(), model.document().text().to_string());
    }
  }

  #[test]
  fn machine_selection_in_zhulu_zhongyuan() {
    gvb_interp::machine::init_machines_from_str(include_str!(
      "../../gvb_interp/machines.yaml"
    ))
    .expect("bundled machines must be valid");
    let document =
      Document::load(include_bytes!("../../examples/bas/逐鹿中原.txt"), false)
        .expect("example loads");
    let mut model = EditorModel::new();
    model.replace_document(document);
    let selected = if model.document().machine_name() == "TC808" {
      "PC1000A"
    } else {
      "TC808"
    };
    model
      .set_machine_name(selected)
      .expect("example switches machine");
    assert_eq!(model.document().machine_name(), selected);
    assert!(
      model
        .source()
        .lines()
        .next()
        .unwrap()
        .contains(&format!("{{type:{selected}}}"))
    );
  }
}
