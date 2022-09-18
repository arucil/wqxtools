#include "code_editor.h"

#include <QContextMenuEvent>
#include <QToolTip>
#include <QUrl>
#include <QtMath>
#include <utility>

#include "../syntax_style.h"
#include "../message_bus.h"

#define INDICATOR_WARNING 0
#define INDICATOR_ERROR 1
#define INDICATOR_RUNTIME_ERROR 2
#define MARKER_WARNING 1
#define MARKER_ERROR 2
#define MARGIN_MARKER 0
#define MARGIN_LINENUMBER 1
#define STYLE_RUNTIME_ERROR (STYLE_LASTPREDEFINED + 1)

CodeEditor::CodeEditor(QWidget *parent) :
  ScintillaEdit(parent),
  m_dirty(),
  m_braceHilit() {
  connect(this, &ScintillaEdit::notify, this, &CodeEditor::notified);
  connect(
    this,
    &ScintillaEdit::savePointChanged,
    this,
    &CodeEditor::dirtyChanged);

  setModEventMask(SC_MOD_INSERTTEXT | SC_MOD_DELETETEXT);

  setMargins(2);
  setMarginTypeN(MARGIN_MARKER, SC_MARGIN_SYMBOL);
  setMarginMaskN(MARGIN_MARKER, SC_MARK_BACKGROUND);
  setMarginWidthN(MARGIN_MARKER, 6);

  setMarginTypeN(MARGIN_LINENUMBER, SC_MARGIN_NUMBER);
  setMarginMaskN(MARGIN_LINENUMBER, 0);

  setMouseDwellTime(400);

  markerDefine(MARKER_WARNING, SC_MARK_FULLRECT);
  markerDefine(MARKER_ERROR, SC_MARK_FULLRECT);

  indicSetStyle(INDICATOR_RUNTIME_ERROR, INDIC_STRAIGHTBOX);

  setWordChars(
    "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789");

  usePopUp(SC_POPUP_NEVER);
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
      int charsAdded = 0;
      size_t pos = static_cast<size_t>(data->position);

      if (bits & SC_MOD_INSERTTEXT) {
        TextChange change;
        change.kind = TextChangeKind::InsertText;
        change.position = pos;
        change.text = data->text;
        charsAdded = data->length;
        change.length = static_cast<size_t>(data->length);
        emit textChanged(change);
      } else if (bits & SC_MOD_DELETETEXT) {
        TextChange change;
        change.kind = TextChangeKind::DeleteText;
        change.position = pos;
        change.text = data->text;
        charsAdded = -data->length;
        change.length = static_cast<size_t>(data->length);
        emit textChanged(change);
      }

      if (data->linesAdded != 0) {
        adjustLineNumberMarginWidth();
      }

      if (m_runtimeError.has_value()) {
        auto &error = m_runtimeError.value();
        if (pos <= error.start) {
          error.start += charsAdded;
          error.end += charsAdded;
          if (data->linesAdded != 0) {
            error.line += data->linesAdded;
            if (annotationLines(error.line) == 0) {
              annotationClearAll();
              annotationSetText(
                error.line,
                error.message.toStdString().c_str());
              annotationSetStyle(error.line, STYLE_RUNTIME_ERROR);
              annotationSetVisible(ANNOTATION_BOXED);
            }
          }
        } else if (pos < error.end) {
          clearRuntimeError();
        }
      }

      break;
    }
    case Scintilla::Notification::DwellStart: {
      if (data->position < 0 || data->position > length()) {
        break;
      }
      auto pos = static_cast<size_t>(data->position);
      showDiagnostics(pos, {data->x, data->y});
      break;
    }
    case Scintilla::Notification::DwellEnd:
      QToolTip::hideText();
      break;
    case Scintilla::Notification::UpdateUI:
      if (
        static_cast<int>(data->updated)
        & (SC_UPDATE_SELECTION | SC_UPDATE_CONTENT)) {
        if (static_cast<int>(data->updated) & SC_UPDATE_SELECTION) {
          emit selectionChanged(!selectionEmpty());
        }
        auto pos = currentPos();
        setTargetRange(selectionStart(), selectionEnd());
        emit cursorPositionChanged(pos);
        auto ch = charAt(pos);
        if (ch == '(' || ch == ')') {
          auto bracePos = braceMatch(pos, 0);
          if (bracePos >= 0) {
            braceHighlight(pos, bracePos);
          } else {
            braceBadLight(pos);
          }
          m_braceHilit = true;
        } else if (m_braceHilit) {
          m_braceHilit = false;
          braceBadLight(-1);
        }
      }
      break;
    case Scintilla::Notification::URIDropped: {
      QUrl url(QString::fromUtf8(data->text));
      if (url.isLocalFile()) {
        emit fileDropped(url.toLocalFile());
      }
      break;
    }
    case Scintilla::Notification::Key: {
      printf("%d\n", data->ch);
      break;
    }
    default:
      break;
  }
}

void CodeEditor::showDiagnostics(size_t pos, const QPoint &p) {
  QString text;
  m_diagRanges.overlap_find_all({pos, pos}, [&text, this](auto it) {
    const auto &diag = m_diagnostics[it->interval().index];
    if (!text.isEmpty()) {
      text += "<hr>";
    }
    // NOTE <nobr> does not work in QToolTip. See
    // https://doc.qt.io/qt-5/qtooltip.html#details
    text += "<p style=\"margin: 0; white-space:pre\">";
    switch (diag.severity) {
      case api::GvbSeverity::Warning: {
        text +=
          "<img style=\"vertical-align: middle;\" src=\":/images/Warning.svg\">"
          "&nbsp;&nbsp;";
        break;
      }
      case api::GvbSeverity::Error: {
        text +=
          "<img style=\"vertical-align: middle;\" src=\":/images/Error.svg\">"
          "&nbsp;&nbsp;";
        break;
      }
    }
    text += diag.message.toHtmlEscaped();
    text += "</p>";
    return true;
  });

  if (text.isEmpty())
    QToolTip::hideText();
  else
    QToolTip::showText(mapToGlobal(p), text);
}

static unsigned abgr(const QColor &color) {
  int r, g, b, a;
  color.getRgb(&r, &g, &b, &a);
  return static_cast<unsigned>(a << 24 | b << 16 | g << 8 | r);
}

void CodeEditor::setStyle(const SyntaxStyle *style) {
  auto size = styleSize(STYLE_DEFAULT);
  styleResetDefault();
  resetElementColour(SC_ELEMENT_CARET);
  resetElementColour(SC_ELEMENT_SELECTION_TEXT);
  resetElementColour(SC_ELEMENT_SELECTION_BACK);
  resetElementColour(SC_ELEMENT_CARET_LINE_BACK);
  auto defaultFontFamily = styleFont(STYLE_DEFAULT);
  styleSetFont(STYLE_LINENUMBER, defaultFontFamily.data());
  styleSetFont(STYLE_CONTROLCHAR, defaultFontFamily.data());
  styleSetFont(STYLE_RUNTIME_ERROR, defaultFontFamily.data());
  styleSetFont(STYLE_DEFAULT, "WenQuXing");
  styleSetFont(0, "WenQuXing");
  styleSetFont(STYLE_BRACEBAD, "WenQuXing");
  styleSetFont(STYLE_BRACELIGHT, "WenQuXing");
  setFontSize(size);

  if (!style) {
    return;
  }

  if (auto fmt = style->getFormat("Text")) {
    if (fmt->foreground.has_value()) {
      auto color = abgr(fmt->foreground.value());
      styleSetFore(STYLE_DEFAULT, color);
      styleSetFore(0, color);
      setElementColour(SC_ELEMENT_CARET, color);
    }
    if (fmt->background.has_value()) {
      auto color = abgr(fmt->background.value());
      styleSetBack(STYLE_DEFAULT, color);
      styleSetBack(0, color);
    }
  }

  auto defaultFore = styleFore(STYLE_DEFAULT);
  auto defaultBack = styleBack(STYLE_DEFAULT);

  if (auto fmt = style->getFormat("Selection")) {
    if (fmt->foreground.has_value()) {
      auto color = abgr(fmt->foreground.value());
      setElementColour(SC_ELEMENT_SELECTION_TEXT, color);
    }
    if (fmt->background.has_value()) {
      auto color = abgr(fmt->background.value());
      setElementColour(SC_ELEMENT_SELECTION_BACK, color);
    }
  }

  if (auto fmt = style->getFormat("LineNumber")) {
    if (fmt->foreground.has_value()) {
      auto color = abgr(fmt->foreground.value());
      styleSetFore(STYLE_LINENUMBER, color);
    } else {
      styleSetFore(STYLE_LINENUMBER, defaultFore);
    }
    if (fmt->background.has_value()) {
      auto color = abgr(fmt->background.value());
      styleSetBack(STYLE_LINENUMBER, color);
    } else {
      styleSetBack(STYLE_LINENUMBER, defaultBack);
    }
  }

  if (auto fmt = style->getFormat("CurrentLine")) {
    if (fmt->background.has_value()) {
      auto color = abgr(fmt->background.value());
      setElementColour(SC_ELEMENT_CARET_LINE_BACK, color);
    }
  }

  if (auto fmt = style->getFormat("Parentheses")) {
    if (fmt->foreground.has_value()) {
      auto color = abgr(fmt->foreground.value());
      styleSetFore(STYLE_BRACELIGHT, color);
    } else {
      styleSetFore(STYLE_BRACELIGHT, defaultFore);
    }
    if (fmt->background.has_value()) {
      auto color = abgr(fmt->background.value());
      styleSetBack(STYLE_BRACELIGHT, color);
    } else {
      styleSetBack(STYLE_BRACELIGHT, defaultBack);
    }
    styleSetBold(STYLE_BRACELIGHT, fmt->bold);
    styleSetItalic(STYLE_BRACELIGHT, fmt->italic);
  }

  if (auto fmt = style->getFormat("ParenthesesMismatch")) {
    if (fmt->foreground.has_value()) {
      auto color = abgr(fmt->foreground.value());
      styleSetFore(STYLE_BRACEBAD, color);
    } else {
      styleSetFore(STYLE_BRACEBAD, defaultFore);
    }
    if (fmt->background.has_value()) {
      auto color = abgr(fmt->background.value());
      styleSetBack(STYLE_BRACEBAD, color);
    } else {
      styleSetBack(STYLE_BRACEBAD, defaultBack);
    }
    styleSetBold(STYLE_BRACEBAD, fmt->bold);
    styleSetItalic(STYLE_BRACEBAD, fmt->italic);
  }

  if (auto fmt = style->getFormat("Warning")) {
    if (fmt->underlineStyle.has_value()) {
      indicSetStyle(
        INDICATOR_WARNING,
        static_cast<unsigned>(fmt->underlineStyle.value()));
      indicSetStrokeWidth(INDICATOR_WARNING, 120);
      indicSetUnder(INDICATOR_WARNING, true);
    } else {
      indicSetUnder(INDICATOR_WARNING, false);
    }
    if (fmt->underlineColor.has_value()) {
      auto color = abgr(fmt->underlineColor.value());
      indicSetFore(INDICATOR_WARNING, color);
      markerSetBack(MARKER_WARNING, color);
    }
  }

  if (auto fmt = style->getFormat("Error")) {
    if (fmt->underlineStyle.has_value()) {
      indicSetStyle(
        INDICATOR_ERROR,
        static_cast<unsigned>(fmt->underlineStyle.value()));
      indicSetStrokeWidth(INDICATOR_ERROR, 120);
      indicSetUnder(INDICATOR_ERROR, true);
    } else {
      indicSetUnder(INDICATOR_ERROR, false);
    }
    if (fmt->underlineColor.has_value()) {
      auto color = abgr(fmt->underlineColor.value());
      indicSetFore(INDICATOR_ERROR, color);
      indicSetFore(INDICATOR_RUNTIME_ERROR, color);
      styleSetFore(STYLE_RUNTIME_ERROR, color);
      markerSetBack(MARKER_ERROR, color);
    }
  }

  styleSetBack(STYLE_RUNTIME_ERROR, defaultBack);
  indicSetAlpha(INDICATOR_RUNTIME_ERROR, 50);
  indicSetOutlineAlpha(INDICATOR_RUNTIME_ERROR, 70);
}

void CodeEditor::setFontSize(unsigned size) {
  styleSetSize(STYLE_DEFAULT, size);
  styleSetSize(STYLE_BRACEBAD, size);
  styleSetSize(STYLE_BRACELIGHT, size);
  styleSetSize(0, size);
}

void CodeEditor::adjustLineNumberMarginWidth() {
  auto digits = qMax(
    static_cast<size_t>(qLn(lineCount() + 1) / M_LN10),
    static_cast<size_t>(1));
  auto digitWidth = textWidth(STYLE_LINENUMBER, "9") * digits;
  setMarginWidthN(MARGIN_LINENUMBER, digitWidth + 16);
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
  markerDeleteAll(MARKER_WARNING);
  markerDeleteAll(MARKER_ERROR);
  for (auto &diag : m_diagnostics) {
    switch (diag.severity) {
      case api::GvbSeverity::Warning:
        setIndicatorCurrent(INDICATOR_WARNING);
        markerAdd(diag.line, MARKER_WARNING);
        break;
      case api::GvbSeverity::Error:
        setIndicatorCurrent(INDICATOR_ERROR);
        markerAdd(diag.line, MARKER_ERROR);
        break;
    }
    if (diag.start == diag.end) {
      indicatorFillRange(diag.start, 1);
    } else {
      indicatorFillRange(diag.start, diag.end - diag.start);
    }
  }
}

void CodeEditor::setRuntimeError(const Diagnostic &error) {
  clearRuntimeError();
  m_runtimeError = error;
  setIndicatorCurrent(INDICATOR_RUNTIME_ERROR);
  indicatorFillRange(error.start, error.end - error.start);
  annotationSetText(error.line, error.message.toStdString().c_str());
  annotationSetStyle(error.line, STYLE_RUNTIME_ERROR);
  annotationSetVisible(ANNOTATION_BOXED);
}

void CodeEditor::clearRuntimeError() {
  m_runtimeError.reset();
  setIndicatorCurrent(INDICATOR_RUNTIME_ERROR);
  indicatorClearRange(0, length());
  annotationClearAll();
}

void CodeEditor::setSearchMatchCase(bool b) {
  auto f = searchFlags();
  if (b) {
    f |= SCFIND_MATCHCASE;
  } else {
    f &= ~SCFIND_MATCHCASE;
  }
  setSearchFlags(f);
}

void CodeEditor::setSearchWholeWord(bool b) {
  auto f = searchFlags();
  if (b) {
    f |= SCFIND_WHOLEWORD;
  } else {
    f &= ~SCFIND_WHOLEWORD;
  }
  setSearchFlags(f);
}

void CodeEditor::setSearchRegExp(bool b) {
  auto f = searchFlags();
  if (b) {
    f |= SCFIND_REGEXP;
  } else {
    f &= ~SCFIND_REGEXP;
  }
  setSearchFlags(f);
}

void CodeEditor::setSearchText(const QString &text) {
  m_searchText = text.toStdString();
}

void CodeEditor::setReplaceText(const QString &text) {
  m_replaceText = text.toStdString();
}

bool CodeEditor::findNext() {
  setTargetRange(currentPos(), length());
  auto pos = searchInTarget(m_searchText.size(), m_searchText.data());
  if (pos < 0) {
    MessageBus::instance()->postMessage("从头开始查找", 600, MessageType::Info);
    targetWholeDocument();
    pos = searchInTarget(m_searchText.size(), m_searchText.data());
    if (pos < 0) {
      MessageBus::instance()->postMessage("没有找到", 600, MessageType::Error);
      return false;
    }
  }
  gotoPos(pos);
  setCurrentPos(targetEnd());
  return true;
}

void CodeEditor::findPrevious() {
  setTargetRange(currentPos() - 1, 0);
  auto pos = searchInTarget(m_searchText.size(), m_searchText.data());
  if (pos < 0) {
    MessageBus::instance()->postMessage("从末尾开始查找", 600, MessageType::Info);
    setTargetRange(length() - 1, 0);
    pos = searchInTarget(m_searchText.size(), m_searchText.data());
    if (pos < 0) {
      MessageBus::instance()->postMessage("没有找到", 600, MessageType::Error);
      return;
    }
  }
  gotoPos(pos);
  setCurrentPos(targetEnd());
}

void CodeEditor::replace() {
  if (targetStart() == targetEnd()) {
    if (!findNext()) {
      return;
    }
  }
  if (searchFlags() & SCFIND_REGEXP) {
    replaceTargetRE(m_replaceText.size(), m_replaceText.data());
  } else {
    replaceTarget(m_replaceText.size(), m_replaceText.data());
  }
  findNext();
}

void CodeEditor::replaceAll() {
  targetWholeDocument();
  beginUndoAction();
  if (searchFlags() & SCFIND_REGEXP) {
    for (;;) {
      auto pos = searchInTarget(m_searchText.size(), m_searchText.data());
      if (pos < 0) {
        break;
      }
      auto len = replaceTargetRE(m_replaceText.size(), m_replaceText.data());
      setTargetRange(targetStart() + len, length());
    }
  } else {
    for (;;) {
      auto pos = searchInTarget(m_searchText.size(), m_searchText.data());
      if (pos < 0) {
        break;
      }
      auto len = replaceTarget(m_replaceText.size(), m_replaceText.data());
      setTargetRange(targetStart() + len, length());
    }
  }
  endUndoAction();
}

void CodeEditor::contextMenuEvent(QContextMenuEvent *event) {
  emit contextMenu(event->pos());
}

void CodeEditor::keyPressEvent(QKeyEvent *event) {
  if (event->key() == Qt::Key_Escape && event->modifiers() == Qt::NoModifier) {
    if (!callTipActive() && !autoCActive() && selections() == 1) {
      emit escape();
    }
  }
  ScintillaEdit::keyPressEvent(event);
}