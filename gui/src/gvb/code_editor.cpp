#include "code_editor.h"

#include <QToolTip>
#include <QtMath>
#include <utility>

#define INDICATOR_WARNING 0
#define INDICATOR_ERROR 1
#define WARNING_COLOR 0x0e'c1'ff
#define ERROR_COLOR 0x30'2e'd3

CodeEditor::CodeEditor(QWidget *parent) :
  ScintillaEdit(parent),
  m_dirty(false) {
  connect(this, &ScintillaEdit::notify, this, &CodeEditor::notified);
  connect(
    this,
    &ScintillaEdit::savePointChanged,
    this,
    &CodeEditor::dirtyChanged);

  setModEventMask(SC_MOD_INSERTTEXT | SC_MOD_DELETETEXT);

  auto defaultFontFamily = m_edit->styleFont(STYLE_DEFAULT);
  auto defaultFontSize = m_edit->styleSize(STYLE_DEFAULT);

  m_edit->styleSetFont(STYLE_LINENUMBER, defaultFontFamily.data());
  // m_edit->styleSetSize(STYLE_LINENUMBER, defaultFontSize);
  m_edit->styleSetFore(STYLE_LINENUMBER, 0xff'a4'72'62);
  m_edit->styleSetBack(STYLE_LINENUMBER, 0xff'ef'ef'ef);

  m_edit->styleSetFont(STYLE_CALLTIP, defaultFontFamily.data());
  m_edit->styleSetSize(STYLE_CALLTIP, defaultFontSize);

  m_edit->styleSetFont(STYLE_DEFAULT, "WenQuXing");
  m_edit->styleSetSize(STYLE_DEFAULT, 12);

  m_edit->styleSetFont(0, "WenQuXing");
  m_edit->styleSetSize(0, 12);

  m_edit->setMarginTypeN(2, SC_MARGIN_NUMBER);

  m_edit->setElementColour(SC_ELEMENT_CARET_LINE_BACK, 0xff'f6'ee'e0);

  m_edit->indicSetStyle(INDICATOR_WARNING, INDIC_SQUIGGLE);
  m_edit->indicSetFore(INDICATOR_WARNING, WARNING_COLOR);
  m_edit->indicSetStrokeWidth(INDICATOR_WARNING, 150);
  m_edit->indicSetHoverStyle(INDICATOR_WARNING, INDIC_FULLBOX);
  m_edit->indicSetHoverFore(INDICATOR_WARNING, WARNING_COLOR);
  m_edit->indicSetOutlineAlpha(INDICATOR_WARNING, 50);
  m_edit->indicSetAlpha(INDICATOR_WARNING, 50);
  m_edit->indicSetUnder(INDICATOR_WARNING, true);

  m_edit->indicSetStyle(INDICATOR_ERROR, INDIC_SQUIGGLE);
  m_edit->indicSetFore(INDICATOR_ERROR, ERROR_COLOR);
  m_edit->indicSetStrokeWidth(INDICATOR_ERROR, 120);
  m_edit->indicSetHoverStyle(INDICATOR_ERROR, INDIC_FULLBOX);
  m_edit->indicSetHoverFore(INDICATOR_ERROR, ERROR_COLOR);
  m_edit->indicSetOutlineAlpha(INDICATOR_ERROR, 70);
  m_edit->indicSetAlpha(INDICATOR_ERROR, 70);
  m_edit->indicSetUnder(INDICATOR_ERROR, true);

  m_edit->setMouseDwellTime(400);
}

void CodeEditor::notified(Scintilla::NotificationData *data) {
  switch (data->nmhdr.code) {
    case Scintilla::Notification::SavePointReached:
      if (m_dirty != false) {
        m_dirty = false;
        emit dirtyChanged(false);
      }
      break;
    case Scintilla::Notification::SavePointLeft:
      if (m_dirty != true) {
        m_dirty = true;
        emit dirtyChanged(true);
      }
      break;
    case Scintilla::Notification::Modified: {
      auto bits = static_cast<int>(data->modificationType);

      if (bits & SC_MOD_INSERTTEXT) {
        TextChange change;
        change.kind = TextChangeKind::InsertText;
        change.position = static_cast<size_t>(data->position);
        change.text = data->text;
        change.length = static_cast<size_t>(data->length);
        emit textChanged(change);
      } else if (bits & SC_MOD_DELETETEXT) {
        TextChange change;
        change.kind = TextChangeKind::DeleteText;
        change.position = static_cast<size_t>(data->position);
        change.text = data->text;
        change.length = static_cast<size_t>(data->length);
        emit textChanged(change);
      }

      if (data->linesAdded != 0) {
        adjustLineNumberMarginWidth();
      }

      break;
    }
    case Scintilla::Notification::DwellStart: {
      if (data->position < 0 || data->position > length()) {
        break;
      }
      auto pos = static_cast<size_t>(data->position);
      std::string messages;
      m_diagRanges.overlap_find_all({pos, pos}, [&messages, this](auto it) {
        if (!messages.empty()) {
          messages += '\n';
        }
        messages += "▸ ";
        messages += m_diagnostics[it->interval().index].message.c_str();
        return true;
      });
      if (!messages.empty()) {
        m_edit->callTipShow(data->position, messages.c_str());
      }
      break;
    }
    case Scintilla::Notification::DwellEnd:
      QToolTip::hideText();
      break;
    case Scintilla::Notification::UpdateUI:
      if (
        static_cast<int>(data->updated)
        & (SC_UPDATE_SELECTION | SC_UPDATE_CONTENT)) {
        emit cursorPositionChanged(currentPos());
      }
      break;
    default:
      break;
  }
}

void CodeEditor::adjustLineNumberMarginWidth() {
  auto digits = qMax(
    static_cast<size_t>(qLn(lineCount() + 1) / M_LN10),
    static_cast<size_t>(1));
  auto digitWidth = textWidth(STYLE_LINENUMBER, "9") * digits;
  setMarginWidthN(2 /* TODO ??? */, digitWidth);
}

void CodeEditor::setDiagnostics(QVector<Diagnostic> diags) {
  m_diagnostics = std::move(diags);

  m_diagRanges.clear();
  for (int i = 0; i < m_diagnostics.size(); i++) {
    auto &diag = m_diagnostics[i];
    Range r = {diag.start, diag.end};
    if (diag.start == diag.end) {
      r = {diag.start, diag.end + 1};
    }
    r.index = static_cast<size_t>(i);
    m_diagRanges.insert(r);
  }

  auto len = m_edit->length();
  m_edit->setIndicatorCurrent(INDICATOR_WARNING);
  m_edit->indicatorClearRange(0, len);
  m_edit->setIndicatorCurrent(INDICATOR_ERROR);
  m_edit->indicatorClearRange(0, len);
  for (auto &diag : m_diagnostics) {
    switch (diag.severity) {
      case api::GvbSeverity::Warning:
        m_edit->setIndicatorCurrent(INDICATOR_WARNING);
        break;
      case api::GvbSeverity::Error:
        m_edit->setIndicatorCurrent(INDICATOR_ERROR);
        break;
    }
    if (diag.start == diag.end) {
      m_edit->indicatorFillRange(diag.start, 1);
    } else {
      m_edit->indicatorFillRange(diag.start, diag.end - diag.start);
    }
  }
}