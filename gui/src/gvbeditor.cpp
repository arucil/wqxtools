#include "gvbeditor.h"
#include "util.h"
#include <QApplication>
#include <QByteArray>
#include <QFileInfo>
#include <QMessageBox>
#include <QThread>
#include <QTimer>
#include <QToolBar>
#include <QVBoxLayout>
#include <ScintillaEdit.h>
#include <algorithm>
#include <cmath>
#include <string>
#include <utility>

#define INDICATOR_WARNING 0
#define INDICATOR_ERROR 1

GvbEditor::GvbEditor(QWidget *parent)
    : Tool(parent), m_doc(nullptr), m_textLoaded(false), m_timerModify(0) {
  initUi();

  connect(
      this, &GvbEditor::updateDiagnostics, this, &GvbEditor::diagnosticsUpdated,
      Qt::QueuedConnection);

  QTimer::singleShot(0, [this] {
    m_pasteEnabled.setValue(true);
    m_undoEnabled.setValue(false);
    m_redoEnabled.setValue(false);
    m_copyCutEnabled.setValue(true);
  });
}

GvbEditor::~GvbEditor() {
  if (m_doc) {
    gvb::destroy_document(m_doc);
    m_doc = nullptr;
  }
}

void GvbEditor::initUi() {
  auto layout = new QVBoxLayout(this);
  auto toolbar = initToolBar();
  initEdit();

  layout->addWidget(toolbar);
  layout->addWidget(m_edit, 1);
  layout->setContentsMargins(0, 0, 0, 0);
  layout->setSpacing(0);
}

void GvbEditor::initEdit() {
  m_edit = new ScintillaEdit(this);

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

  m_edit->setModEventMask(SC_MOD_INSERTTEXT | SC_MOD_DELETETEXT);

  m_edit->setElementColour(SC_ELEMENT_CARET_LINE_BACK, 0xff'f6'ee'e0);
  m_edit->setCaretLineVisibleAlways(true);

  m_edit->setEOLMode(SC_EOL_CRLF);

  m_edit->indicSetStyle(INDICATOR_WARNING, INDIC_SQUIGGLELOW);
  // TODO color
  m_edit->indicSetFore(INDICATOR_WARNING, 0x2d'cd'e2);
  m_edit->indicSetStrokeWidth(INDICATOR_ERROR, 200);
  m_edit->indicSetStyle(INDICATOR_ERROR, INDIC_SQUIGGLELOW);
  m_edit->indicSetFore(INDICATOR_ERROR, 0x2c'3e'f5);
  m_edit->indicSetStrokeWidth(INDICATOR_WARNING, 150);
  m_edit->callTipUseStyle(0);

  m_edit->setMouseDwellTime(400);

  connect(m_edit, &ScintillaEdit::notify, this, &GvbEditor::notified);

  connect(m_edit, &ScintillaEdit::savePointChanged, this, [this](bool dirty) {
    m_dirty.setValue(dirty);
  });
}

QToolBar *GvbEditor::initToolBar() {
  auto toolbar = new QToolBar;

  m_actSave = toolbar->addAction(QPixmap(":/assets/images/Save.svg"), "保存");

  toolbar->addSeparator();

  m_actStart = new QAction(QPixmap(":/assets/images/Run.svg"), "运行");
  toolbar->addAction(m_actStart);

  m_actStop = new QAction(QPixmap(":/assets/images/Stop.svg"), "停止");
  toolbar->addAction(m_actStop);

  toolbar->addSeparator();

  auto actFind =
      toolbar->addAction(QPixmap(":/assets/images/Find.svg"), "查找");
  connect(actFind, &QAction::triggered, this, &GvbEditor::find);

  auto actReplace =
      toolbar->addAction(QPixmap(":/assets/images/Replace.svg"), "替换");
  connect(actReplace, &QAction::triggered, this, &GvbEditor::replace);

  toolbar->addSeparator();

  auto actUndo =
      toolbar->addAction(QPixmap(":/assets/images/Undo.svg"), "撤销");
  connect(actUndo, &QAction::triggered, this, &GvbEditor::undo);
  connect(&m_undoEnabled, &BoolValue::changed, actUndo, &QAction::setEnabled);

  auto actRedo =
      toolbar->addAction(QPixmap(":/assets/images/Redo.svg"), "重做");
  connect(actRedo, &QAction::triggered, this, &GvbEditor::redo);
  connect(&m_redoEnabled, &BoolValue::changed, actRedo, &QAction::setEnabled);

  toolbar->addSeparator();

  auto actCopy =
      toolbar->addAction(QPixmap(":/assets/images/Copy.png"), "复制");
  connect(actCopy, &QAction::triggered, this, &GvbEditor::copy);
  connect(
      &m_copyCutEnabled, &BoolValue::changed, actCopy, &QAction::setEnabled);

  auto actCut = toolbar->addAction(QPixmap(":/assets/images/Cut.svg"), "剪切");
  connect(actCut, &QAction::triggered, this, &GvbEditor::cut);
  connect(&m_copyCutEnabled, &BoolValue::changed, actCut, &QAction::setEnabled);

  auto actPaste =
      toolbar->addAction(QPixmap(":/assets/images/Paste.png"), "粘贴");
  connect(actPaste, &QAction::triggered, this, &GvbEditor::paste);
  connect(&m_pasteEnabled, &BoolValue::changed, actPaste, &QAction::setEnabled);

  return toolbar;
}

ActionResult GvbEditor::save(const QString &) {
  m_edit->setSavePoint();
  // TODO
  return ActionResult::Succeed;
}

void GvbEditor::create() {
  // TODO
}

ActionResult GvbEditor::load(const QString &path) {
  auto result =
      gvb::load_document({path.utf16(), static_cast<size_t>(path.size())});
  if (result.tag == gvb::Either<gvb::Utf8String, gvb::Document *>::Tag::Left) {
    auto msg = result.left._0;
    QMessageBox::critical(
        getMainWindow(), "文件打开失败", QString::fromUtf8(msg.data, msg.len));
    gvb::destroy_string(msg);
    return ActionResult::Fail;
  } else {
    if (m_doc) {
      gvb::destroy_document(m_doc);
    }

    m_doc = result.right._0;

    auto text = gvb::document_text(m_doc);

    m_edit->setText(std::string(text.data, text.len).c_str());
    m_textLoaded = true;
    m_edit->setSavePoint();
    m_edit->emptyUndoBuffer();

    auto digits = static_cast<size_t>(
        std::log10(std::count(text.data, text.data + text.len, '\n') + 1));
    auto digitWidth = m_edit->textWidth(STYLE_LINENUMBER, "9") * digits;
    m_edit->setMarginWidthN(2, digitWidth);

    gvb::destroy_string(text);

    computeDiagnostics();

    return ActionResult::Succeed;
  }
}

bool GvbEditor::canLoad(const QString &path) const {
  auto ext = QFileInfo(path).suffix().toLower();
  return ext == "bas" || ext == "txt";
}

void GvbEditor::find() {
  // TODO
}

void GvbEditor::replace() {
  // TODO
}

void GvbEditor::cut() {
  m_edit->cut();
}

void GvbEditor::copy() {
  m_edit->copy();
}

void GvbEditor::paste() {
  m_edit->paste();
}

void GvbEditor::undo() {
  m_edit->undo();
}

void GvbEditor::redo() {
  m_edit->redo();
}

void GvbEditor::notified(Scintilla::NotificationData *data) {
  switch (data->nmhdr.code) {
  case Scintilla::Notification::SavePointReached:
    m_dirty.setValue(false);
    break;
  case Scintilla::Notification::SavePointLeft:
    m_dirty.setValue(true);
    break;
  case Scintilla::Notification::Modified: {
    if (!m_textLoaded) {
      return;
    }

    auto bits = static_cast<int>(data->modificationType);

    if (!m_timerModify) {
      m_timerModify = startTimer(500);
    }

    m_undoEnabled.setValue(m_edit->canUndo());
    m_redoEnabled.setValue(m_edit->canRedo());
    if (bits & SC_MOD_INSERTTEXT) {
      InsertText *insert;
      if (!m_edits.empty() &&
          (insert = std::get_if<InsertText>(&m_edits.back())) &&
          insert->pos + insert->str.size() ==
              static_cast<size_t>(data->position)) {
        insert->str.append(data->text, data->length);
      } else {
        InsertText insert = {
            static_cast<size_t>(data->position),
            std::string(data->text, data->length)};
        m_edits.push_back(insert);
      }
    } else if (bits & SC_MOD_DELETETEXT) {
      DeleteText *del;
      if (!m_edits.empty() &&
          (del = std::get_if<DeleteText>(&m_edits.back())) &&
          del->pos == static_cast<size_t>(data->position + data->length)) {
        del->len += static_cast<size_t>(data->length);
        del->pos -= static_cast<size_t>(data->length);
      } else {
        DeleteText del = {
            static_cast<size_t>(data->position),
            static_cast<size_t>(data->length)};
        m_edits.push_back(del);
      }
    }
    break;
  }
  case Scintilla::Notification::DwellStart:
        // TODO efficient range query
    for (const auto &diag : m_diagnostics) {
      if (data->position >= diag.start && data->position < diag.end) {
        m_edit->callTipShow(data->position, diag.message.c_str());
        break;
      }
    }
    break;
  case Scintilla::Notification::DwellEnd:
    m_edit->callTipCancel();
    break;
  default:
    break;
  }
}

void GvbEditor::timerEvent(QTimerEvent *event) {
  if (event->timerId() == m_timerModify) {
    for (auto edit : m_edits) {
      if (auto insert = std::get_if<InsertText>(&edit)) {
        gvb::Modification ins = {
            gvb::Modification::Tag::Left,
            {insert->pos, {insert->str.c_str(), insert->str.size()}}};
        gvb::document_apply_edit(m_doc, ins);
      } else {
        auto del = std::get<DeleteText>(edit);
        gvb::Modification d = {
            gvb::Modification::Tag::Right,
        };
        d.right._0.pos = del.pos;
        d.right._0.len = del.len;
        gvb::document_apply_edit(m_doc, d);
      }
    }
    m_edits.clear();

    computeDiagnostics();
  }
}

void GvbEditor::diagnosticsUpdated(std::vector<Diagnostic> diags) {
  m_diagnostics = std::move(diags);

  m_edit->indicatorClearRange(0, m_edit->length());
  for (auto &diag : m_diagnostics) {
    switch (diag.severity) {
    case gvb::Severity::Warning:
      m_edit->setIndicatorCurrent(INDICATOR_WARNING);
      break;
    case gvb::Severity::Error:
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

void GvbEditor::computeDiagnostics() {
  auto thread = QThread::create([this] {
    auto diags = gvb::document_diagnostics(m_doc);
    std::vector<Diagnostic> diagVec;
    for (auto it = diags.data; it < diags.data + diags.len; it++) {
      Diagnostic d = {
          it->line,
          it->start,
          it->end,
          it->severity,
          std::string(it->message.data, it->message.len),
      };
      diagVec.push_back(d);
    }
    emit(updateDiagnostics(diagVec));
  });
  thread->start();
  connect(thread, &QThread::finished, thread, &QObject::deleteLater);
}