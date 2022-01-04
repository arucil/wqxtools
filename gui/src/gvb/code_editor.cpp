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

  auto defaultFontFamily = styleFont(STYLE_DEFAULT);
  auto defaultFontSize = styleSize(STYLE_DEFAULT);

  styleSetFont(STYLE_LINENUMBER, defaultFontFamily.data());
  // m_edit->styleSetSize(STYLE_LINENUMBER, defaultFontSize);
  styleSetFore(STYLE_LINENUMBER, 0xff'a4'72'62);
  styleSetBack(STYLE_LINENUMBER, 0xff'ef'ef'ef);

  styleSetFont(STYLE_DEFAULT, "WenQuXing");
  styleSetSize(STYLE_DEFAULT, 12);

  styleSetFont(0, "WenQuXing");
  styleSetSize(0, 12);

  setMarginTypeN(2, SC_MARGIN_NUMBER);

  setElementColour(SC_ELEMENT_CARET_LINE_BACK, 0xff'f6'ee'e0);

  indicSetStyle(INDICATOR_WARNING, INDIC_SQUIGGLE);
  indicSetFore(INDICATOR_WARNING, WARNING_COLOR);
  indicSetStrokeWidth(INDICATOR_WARNING, 150);
  indicSetUnder(INDICATOR_WARNING, true);

  indicSetStyle(INDICATOR_ERROR, INDIC_SQUIGGLE);
  indicSetFore(INDICATOR_ERROR, ERROR_COLOR);
  indicSetStrokeWidth(INDICATOR_ERROR, 150);
  indicSetUnder(INDICATOR_ERROR, true);

  setMouseDwellTime(400);
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
      QString text;
      m_diagRanges.overlap_find_all({pos, pos}, [&text, this](auto it) {
        const auto &diag = m_diagnostics[it->interval().index];
        if (!text.isEmpty()) {
          text += "<hr>";
        }
        // NOTE <nobr> does not work in QToolTip. See
        // https://doc.qt.io/qt-5/qtooltip.html#details
        text += "<p style=\"margin: 0; white-space:pre\">";
        text += diag.message.toHtmlEscaped();
        text += "</p>";
        return true;
      });

      if (text.isEmpty())
        QToolTip::hideText();
      else
        QToolTip::showText(mapToGlobal({data->x, data->y}), text);
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

void CodeEditor::setStyle(const SyntaxStyle *style) {}

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

  auto len = length();
  setIndicatorCurrent(INDICATOR_WARNING);
  indicatorClearRange(0, len);
  setIndicatorCurrent(INDICATOR_ERROR);
  indicatorClearRange(0, len);
  for (auto &diag : m_diagnostics) {
    switch (diag.severity) {
      case api::GvbSeverity::Warning:
        setIndicatorCurrent(INDICATOR_WARNING);
        break;
      case api::GvbSeverity::Error:
        setIndicatorCurrent(INDICATOR_ERROR);
        break;
    }
    if (diag.start == diag.end) {
      indicatorFillRange(diag.start, 1);
    } else {
      indicatorFillRange(diag.start, diag.end - diag.start);
    }
  }
}