#include "gvbeditor.h"

#include <QApplication>
#include <QByteArray>
#include <QDir>
#include <QFileInfo>
#include <QLabel>
#include <QMessageBox>
#include <QState>
#include <QTimer>
#include <QToolBar>
#include <QVBoxLayout>
#include <QtMath>

#include "../action.h"
#include "../util.h"
#include "code_editor.h"
#include "gvbsim_window.h"

#define INDICATOR_WARNING 0
#define INDICATOR_ERROR 1
#define WARNING_COLOR 0x0e'c1'ff
#define ERROR_COLOR 0x30'2e'd3
#define DATA_DIR "dat_files"

using std::get_if;
using std::in_place_index;

GvbEditor::GvbEditor(QWidget *parent) :
  Tool(parent),
  m_doc(nullptr),
  m_textLoaded(false),
  m_timerModify(false),
  m_gvbsim(nullptr) {
  initUi();
  initStateMachine();

  QTimer::singleShot(0, this, [this] {
    m_actPaste->setEnabled(true);
    m_actUndo->setEnabled(false);
    m_actRedo->setEnabled(false);
    m_actCopy->setEnabled(true);
    m_actCut->setEnabled(true);
    m_actFind->setEnabled(true);
    m_actReplace->setEnabled(true);

    m_stateMachine.start();
  });
}

GvbEditor::~GvbEditor() {
  if (m_doc) {
    gvb_destroy_document(m_doc);
    m_doc = nullptr;
  }
}

void GvbEditor::initUi() {
  auto layout = new QVBoxLayout(this);
  auto toolbar = initToolBar();
  initEdit();
  auto statusbar = initStatusBar();

  layout->addWidget(toolbar);
  layout->addWidget(m_edit, 1);
  layout->addWidget(statusbar);
  layout->setContentsMargins(0, 0, 0, 0);
  layout->setSpacing(0);
}

void GvbEditor::initStateMachine() {
  m_stStarted = new QState;
  m_stPaused = new QState;
  m_stStopped = new QState;
  m_stStopped->addTransition(this, &GvbEditor::start, m_stStarted);
  m_stStarted->addTransition(this, &GvbEditor::pause, m_stPaused);
  m_stPaused->addTransition(this, &GvbEditor::cont, m_stStarted);
  m_stStarted->addTransition(this, &GvbEditor::stop, m_stStopped);
  m_stPaused->addTransition(this, &GvbEditor::stop, m_stStopped);
  m_stateMachine.addState(m_stStarted);
  m_stateMachine.addState(m_stPaused);
  m_stateMachine.addState(m_stStopped);
  m_stateMachine.setInitialState(m_stStopped);

  connect(m_stStarted, &QState::entered, this, [this] {
    updateStartAction(m_stStarted);
    m_actStop->setEnabled(true);
  });
  connect(m_stStopped, &QState::entered, this, [this] {
    updateStartAction(m_stStopped);
    m_actStop->setEnabled(false);
  });
  connect(m_stPaused, &QState::entered, this, [this] {
    updateStartAction(m_stPaused);
    m_actStop->setEnabled(true);
  });
}

void GvbEditor::updateStartAction(QState *state) {
  if (state == m_stStopped) {
    if (m_gvbsim) {
      m_actStart->setText("运行");
      m_actStart->setIcon(QPixmap(":/assets/images/Run.svg"));
    } else {
      m_actStart->setText("启动模拟器");
      m_actStart->setIcon(QPixmap(":/assets/images/Simulator.svg"));
    }
  } else if (state == m_stPaused) {
    m_actStart->setText("继续");
    m_actStart->setIcon(QPixmap(":/assets/images/Run.svg"));
  } else if (state == m_stStarted) {
    m_actStart->setText("暂停");
    m_actStart->setIcon(QPixmap(":/assets/images/Pause.svg"));
  }
}

void GvbEditor::initEdit() {
  m_edit = new CodeEditor(this);

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

  m_edit->callTipUseStyle(0);

  m_edit->setMouseDwellTime(400);

  connect(m_edit, &ScintillaEdit::notify, this, &GvbEditor::notified);

  connect(
    m_edit,
    &ScintillaEdit::savePointChanged,
    &m_dirty,
    &BoolValue::setValue);
}

QToolBar *GvbEditor::initToolBar() {
  auto toolbar = new QToolBar;
  toolbar->setContextMenuPolicy(Qt::PreventContextMenu);

  m_actSave = toolbar->addAction(QPixmap(":/assets/images/Save.svg"), "保存");

  toolbar->addSeparator();

  m_actFind = new Action(QPixmap(":/assets/images/Find.svg"), "查找");
  toolbar->addAction(m_actFind);
  connect(m_actFind, &QAction::triggered, this, &GvbEditor::find);

  m_actReplace = new Action(QPixmap(":/assets/images/Replace.svg"), "替换");
  toolbar->addAction(m_actReplace);
  connect(m_actReplace, &QAction::triggered, this, &GvbEditor::replace);

  toolbar->addSeparator();

  m_actUndo = new Action(QPixmap(":/assets/images/Undo.svg"), "撤销");
  toolbar->addAction(m_actUndo);
  connect(m_actUndo, &QAction::triggered, this, &GvbEditor::undo);

  m_actRedo = new Action(QPixmap(":/assets/images/Redo.svg"), "重做");
  toolbar->addAction(m_actRedo);
  connect(m_actRedo, &QAction::triggered, this, &GvbEditor::redo);

  toolbar->addSeparator();

  m_actCopy = new Action(QPixmap(":/assets/images/Copy.svg"), "复制");
  toolbar->addAction(m_actCopy);
  connect(m_actCopy, &QAction::triggered, this, &GvbEditor::copy);

  m_actCut = new Action(QPixmap(":/assets/images/Cut.svg"), "剪切");
  toolbar->addAction(m_actCut);
  connect(m_actCut, &QAction::triggered, this, &GvbEditor::cut);

  m_actPaste = new Action(QPixmap(":/assets/images/Paste.svg"), "粘贴");
  toolbar->addAction(m_actPaste);
  connect(m_actPaste, &QAction::triggered, this, &GvbEditor::paste);

  toolbar->addSeparator();

  m_actStart = new Action;
  toolbar->addAction(m_actStart);
  connect(m_actStart, &QAction::triggered, this, [this] {
    tryStartPause(this);
  });

  auto empty = new QWidget();
  empty->setMinimumWidth(20);
  toolbar->addWidget(empty);

  m_actStop = new Action(QPixmap(":/assets/images/Stop.svg"), "停止");
  toolbar->addAction(m_actStop);
  connect(m_actStop, &QAction::triggered, this, &GvbEditor::stop);

  return toolbar;
}

QStatusBar *GvbEditor::initStatusBar() {
  auto statusbar = new QStatusBar;
  auto posLabel = new QLabel;
  posLabel->setMinimumWidth(120);
  statusbar->addPermanentWidget(posLabel);

  connect(
    m_edit,
    &CodeEditor::cursorPositionChanged,
    this,
    [posLabel, this](size_t pos) {
      auto line = m_edit->lineFromPosition(pos) + 1;
      auto col = m_edit->column(pos) + 1;
      posLabel->setText(QString("第 %1 行, 第 %2 列").arg(line, col));
    });

  return statusbar;
}

SaveResult GvbEditor::save(const QString &path) {
  auto saveToPath = path;
  while (true) {
    auto result = gvb_save_document(
      m_doc,
      {saveToPath.utf16(), static_cast<size_t>(saveToPath.size())});
    if (result.tag == api::Either<api::GvbSaveError, api::Unit>::Tag::Left) {
      auto msg = result.left._0.message;
      auto err = QString::fromUtf8(msg.data, msg.len);
      destroy_string(msg);
      if (result.left._0.bas_specific) {
        auto result = QMessageBox::question(
          getMainWindow(),
          "文件保存失败",
          QString("发生错误：%1。无法保存为 .bas 文件，是否保存为 .txt 文件？")
            .arg(err),
          QMessageBox::StandardButton::Yes | QMessageBox::StandardButton::No
            | QMessageBox::StandardButton::Cancel);
        if (result == QMessageBox::StandardButton::Yes) {
          auto info = QFileInfo(saveToPath);
          saveToPath = info.path() + "/" + info.completeBaseName() + ".txt";
          continue;
        } else {
          return SaveResult {in_place_index<2>, Unit {}};
        }
      } else {
        return SaveResult {in_place_index<1>, err};
      }
    } else {
      m_edit->setSavePoint();
      return SaveResult {in_place_index<0>, path};
    }
  }
}

void GvbEditor::create() {
  // TODO
}

LoadResult GvbEditor::load(const QString &path) {
  auto result =
    api::gvb_load_document({path.utf16(), static_cast<size_t>(path.size())});
  if (
    result.tag == api::Either<api::Utf8String, api::GvbDocument *>::Tag::Left) {
    m_filePath.clear();
    auto msg = result.left._0;
    auto err = QString::fromUtf8(msg.data, msg.len);
    api::destroy_string(msg);
    return err;
  } else {
    m_filePath = path;

    if (m_doc) {
      api::gvb_destroy_document(m_doc);
    }

    m_doc = result.right._0;

    auto text = api::gvb_document_text(m_doc);

    m_textLoaded = false;
    m_edit->setText(std::string(text.data, text.len).c_str());
    m_textLoaded = true;
    m_edit->setSavePoint();
    m_edit->emptyUndoBuffer();
    m_edit->setCurrentPos(0);
    m_actUndo->setEnabled(false);
    m_actRedo->setEnabled(false);

    auto digits = qMax(
      static_cast<size_t>(qLn(m_edit->lineCount() + 1) / M_LN10),
      static_cast<size_t>(1));
    auto digitWidth = m_edit->textWidth(STYLE_LINENUMBER, "9") * digits;
    m_edit->setMarginWidthN(2, digitWidth);

    computeDiagnostics();

    return Unit {};
  }
}

bool GvbEditor::canLoad(const QString &path) const {
  auto ext = QFileInfo(path).suffix().toLower();
  return ext == "bas" || ext == "txt";
}

QSize GvbEditor::preferredWindowSize() const {
  return QSize(800, 540);
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

void GvbEditor::modified() {
  for (auto edit : m_edits) {
    if (auto insert = get_if<InsertText>(&edit)) {
      api::GvbModification ins = {
        api::GvbModification::Tag::Left,
        {insert->pos, {insert->str.c_str(), insert->str.size()}}};
      api::gvb_document_apply_edit(m_doc, ins);
    } else {
      auto del = std::get<DeleteText>(edit);
      api::GvbModification d = {
        api::GvbModification::Tag::Right,
      };
      d.right._0.pos = del.pos;
      d.right._0.len = del.len;
      gvb_document_apply_edit(m_doc, d);
    }
  }
  m_edits.clear();

  computeDiagnostics();

  m_timerModify = false;
}

void GvbEditor::diagnosticsUpdated(QVector<Diagnostic> diags) {
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

void GvbEditor::computeDiagnostics() {
  auto diags = gvb_document_diagnostics(m_doc);
  QVector<Diagnostic> diagVec;
  for (auto it = diags.data; it < diags.data + diags.len; it++) {
    Diagnostic d = {
      it->line,
      it->start,
      it->end,
      it->severity,
      QString::fromUtf8(it->message.data, static_cast<int>(it->message.len)),
    };
    diagVec.push_back(d);
  }
  gvb_destroy_str_diagnostic_array(diags);
  m_edit->setDiagnostics(diagVec);
}

void GvbEditor::tryStartPause(QWidget *sender) {
  auto curState = *m_stateMachine.configuration().begin();
  if (curState == m_stStopped) {
    if (sender == this) {
      auto exeDir = QDir(QCoreApplication::applicationDirPath());
      if (!exeDir.exists(DATA_DIR)) {
        exeDir.mkdir(DATA_DIR);
      }
      auto dataDir =
        QDir::cleanPath(exeDir.path() + QDir::separator() + DATA_DIR);
      auto device = gvb_document_device(
        m_doc,
        {dataDir.utf16(), static_cast<size_t>(dataDir.size())});
      auto result = gvb_document_vm(m_doc, device);
      if (result.tag == api::Maybe<api::GvbVirtualMachine *>::Tag::Nothing) {
        // TODO toast
        QMessageBox::critical(getMainWindow(), "错误", "文件有错误，无法运行");
        return;
      }
      auto vm = result.just._0;
      auto newWin = false;
      if (!m_gvbsim) {
        newWin = true;
        m_gvbsim = new GvbSimWindow(getMainWindow(), this);
        m_gvbsim->setAttribute(Qt::WA_DeleteOnClose);
        connect(m_gvbsim, &QMainWindow::destroyed, this, [this] {
          m_gvbsim = nullptr;
          updateStartAction(m_stStopped);
        });
      }
      m_gvbsim->reset(vm, device, QFileInfo(m_filePath).completeBaseName());
      m_gvbsim->show();
      // bring simulator window to front
      m_gvbsim->setWindowState(Qt::WindowState::WindowActive);
      m_gvbsim->raise();
      m_gvbsim->activateWindow();
      if (newWin) {
        updateStartAction(m_stStopped);
      } else {
        emit start();
      }
    } else {
      emit start();
    }
  } else if (curState == m_stPaused) {
    emit cont();
  } else if (curState == m_stStarted) {
    emit pause();
  }
}