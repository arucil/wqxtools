#include "gvbeditor.h"

#include <QApplication>
#include <QByteArray>
#include <QDir>
#include <QFileInfo>
#include <QLabel>
#include <QMessageBox>
#include <QPalette>
#include <QState>
#include <QTimer>
#include <QToolBar>
#include <QToolTip>
#include <QVBoxLayout>
#include <QtMath>
#include <optional>
#include <utility>

#include "../action.h"
#include "../config.h"
#include "../util.h"
#include "code_editor.h"
#include "gvbsim_window.h"

#define DATA_DIR "dat_files"

using std::get_if;
using std::in_place_index;

GvbEditor::GvbEditor(QWidget *parent) :
  Tool(parent),
  m_doc(nullptr),
  m_textLoaded(false),
  m_timerModify(0),
  m_timerError(0),
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
  initToolBar();
  initEdit();
  auto statusbar = initStatusBar();

  layout->addWidget(m_toolbar);
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
    m_edit->clearRuntimeError();
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
      m_actStart->setIcon(QPixmap(":/images/Run.svg"));
    } else {
      m_actStart->setText("启动模拟器");
      m_actStart->setIcon(QPixmap(":/images/Simulator.svg"));
    }
  } else if (state == m_stPaused) {
    m_actStart->setText("继续");
    m_actStart->setIcon(QPixmap(":/images/Run.svg"));
  } else if (state == m_stStarted) {
    m_actStart->setText("暂停");
    m_actStart->setIcon(QPixmap(":/images/Pause.svg"));
  }
}

void GvbEditor::initEdit() {
  m_edit = new CodeEditor(this);

  m_edit->setCaretLineVisibleAlways(true);
  m_edit->setEOLMode(SC_EOL_CRLF);

  m_edit->setFontSize(12);

  connect(m_edit, &CodeEditor::dirtyChanged, &m_dirty, &BoolValue::setValue);
  connect(m_edit, &CodeEditor::textChanged, this, &GvbEditor::textChanged);
  connect(
    &Config::instance(),
    &Config::styleChanged,
    m_edit,
    &CodeEditor::setStyle);
  connect(&Config::instance(), &Config::configChanged, m_edit, [this]() {
    m_edit->setFontSize(api::config()->gvb.editor.font_size);
  });

  m_edit->setStyle(Config::instance().getStyle());
}

void GvbEditor::initToolBar() {
  m_toolbar = new QToolBar;
  m_toolbar->setContextMenuPolicy(Qt::PreventContextMenu);

  m_actSave = m_toolbar->addAction(QPixmap(":/images/Save.svg"), "保存");

  m_toolbar->addSeparator();

  m_actFind = new Action(QPixmap(":/images/Find.svg"), "查找");
  m_toolbar->addAction(m_actFind);
  connect(m_actFind, &QAction::triggered, this, &GvbEditor::find);

  m_actReplace = new Action(QPixmap(":/images/Replace.svg"), "替换");
  m_toolbar->addAction(m_actReplace);
  connect(m_actReplace, &QAction::triggered, this, &GvbEditor::replace);

  m_toolbar->addSeparator();

  m_actUndo = new Action(QPixmap(":/images/Undo.svg"), "撤销");
  m_toolbar->addAction(m_actUndo);
  connect(m_actUndo, &QAction::triggered, this, &GvbEditor::undo);

  m_actRedo = new Action(QPixmap(":/images/Redo.svg"), "重做");
  m_toolbar->addAction(m_actRedo);
  connect(m_actRedo, &QAction::triggered, this, &GvbEditor::redo);

  m_toolbar->addSeparator();

  m_actCopy = new Action(QPixmap(":/images/Copy.svg"), "复制");
  m_toolbar->addAction(m_actCopy);
  connect(m_actCopy, &QAction::triggered, this, &GvbEditor::copy);

  m_actCut = new Action(QPixmap(":/images/Cut.svg"), "剪切");
  m_toolbar->addAction(m_actCut);
  connect(m_actCut, &QAction::triggered, this, &GvbEditor::cut);

  m_actPaste = new Action(QPixmap(":/images/Paste.svg"), "粘贴");
  m_toolbar->addAction(m_actPaste);
  connect(m_actPaste, &QAction::triggered, this, &GvbEditor::paste);

  m_toolbar->addSeparator();

  m_actStart = new Action;
  m_toolbar->addAction(m_actStart);
  connect(m_actStart, &QAction::triggered, this, [this] {
    tryStartPause(this);
  });

  auto empty = new QWidget();
  empty->setMinimumWidth(20);
  m_toolbar->addWidget(empty);

  m_actStop = new Action(QPixmap(":/images/Stop.svg"), "停止");
  m_toolbar->addAction(m_actStop);
  connect(m_actStop, &QAction::triggered, this, &GvbEditor::stop);
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
      posLabel->setText(QString("第 %1 行, 第 %2 列").arg(line).arg(col));
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

void GvbEditor::textChanged(const TextChange &c) {
  if (!m_textLoaded) {
    return;
  }

  if (!m_timerModify) {
    m_timerModify = startTimer(500);
  }

  m_actUndo->setEnabled(m_edit->canUndo());
  m_actRedo->setEnabled(m_edit->canRedo());

  switch (c.kind) {
    case TextChangeKind::InsertText: {
      InsertText *insert;
      if (
        !m_edits.empty() && (insert = get_if<InsertText>(&m_edits.back()))
        && insert->pos + insert->str.size() == c.position) {
        insert->str.append(c.text, c.length);
      } else {
        InsertText insert = {c.position, std::string(c.text, c.length)};
        m_edits.push_back(insert);
      }
      break;
    }
    case TextChangeKind::DeleteText: {
      DeleteText *del;
      if (
        !m_edits.empty() && (del = get_if<DeleteText>(&m_edits.back()))
        && del->pos == c.position + c.length) {
        del->len += c.length;
        del->pos -= c.length;
      } else {
        DeleteText del = {c.position, c.length};
        m_edits.push_back(del);
      }
      break;
    }
  }
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
  m_edit->setDiagnostics(std::move(diagVec));
}

void GvbEditor::tryStartPause(QWidget *sender) {
  auto curState = *m_stateMachine.configuration().begin();
  if (curState == m_stStopped) {
    if (sender == this) {
      auto dataDir = getSystemDir(DATA_DIR);
      auto device = gvb_document_device(
        m_doc,
        {dataDir.utf16(), static_cast<size_t>(dataDir.size())});
      auto result = gvb_document_vm(m_doc, device);
      if (result.tag == api::Maybe<api::GvbVirtualMachine *>::Tag::Nothing) {
        auto btn = m_toolbar->widgetForAction(m_actStart);
        auto btnCenter = btn->size() / 2;
        showErrorToolTip(
          btn->mapToGlobal(QPoint(btnCenter.width(), btnCenter.height())),
          "文件有错误，无法运行");
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

static std::optional<QPalette> orgPalette;

void GvbEditor::showErrorToolTip(const QPoint &pos, const QString &text) {
  if (!orgPalette.has_value()) {
    orgPalette = QToolTip::palette();
  }
  auto pal = orgPalette.value();
  pal.setColor(
    QPalette::ColorGroup::Inactive,
    QPalette::ColorRole::ToolTipText,
    QColor::fromRgb(0xff0000));
  pal.setBrush(
    QPalette::ColorGroup::Inactive,
    QPalette::ColorRole::ToolTipText,
    QColor::fromRgb(0xff0000));
  QToolTip::setPalette(pal);
  QToolTip::showText(pos, text, nullptr, QRect(), 1000);
  if (m_timerError) {
    killTimer(m_timerError);
  }
  m_timerError = startTimer(1000);
}

void GvbEditor::timerEvent(QTimerEvent *ev) {
  if (ev->timerId() == m_timerModify) {
    killTimer(m_timerModify);
    m_timerModify = 0;
    modified();
  } else if (ev->timerId() == m_timerError) {
    killTimer(m_timerError);
    m_timerError = 0;
    QToolTip::setPalette(orgPalette.value());
  }
}

void GvbEditor::showRuntimeError(const api::GvbExecResult::Error_Body &error) {
  auto lineStart = m_edit->positionFromLine(error.location.line);
  auto start = lineStart + error.location.start_column;
  auto end = lineStart + error.location.end_column;
  Diagnostic diag {
    error.location.line,
    start,
    end,
    api::GvbSeverity::Error,
    QString::fromUtf8(error.message.data, error.message.len)};
  m_edit->setRuntimeError(diag);
  m_edit->gotoPos(start);
}