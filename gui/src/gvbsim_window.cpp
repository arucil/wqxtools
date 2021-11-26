#include "gvbsim_window.h"
#include "gvbeditor.h"
#include "gvbsim_keyboard.h"
#include "gvbsim_screen.h"
#include <QDateTime>
#include <QKeyEvent>
#include <QMessageBox>
#include <QTimer>
#include <QToolBar>
#include <QVBoxLayout>

const size_t EXEC_STEPS = 50;

GvbSimWindow::GvbSimWindow(QWidget *parent, GvbEditor *editor)
    : QMainWindow(parent), m_editor(editor), m_vm(nullptr), m_device(nullptr),
      m_paused(false), m_timerCursor(0), m_timerRepaint(0),
      m_state("准备就绪") {
  initUi();

  connect(editor, &GvbEditor::start, this, &GvbSimWindow::start);
  connect(editor, &GvbEditor::stop, this, &GvbSimWindow::stop);
  connect(editor, &GvbEditor::cont, this, &GvbSimWindow::cont);
  connect(editor, &GvbEditor::pause, this, &GvbSimWindow::pause);

  m_execResult.tag = api::GvbExecResult::Tag::Continue;
  m_execInput.tag = api::GvbExecInput::Tag::None;

  adjustSize();
}

void GvbSimWindow::reset() {
  m_paused = false;
  api::gvb_reset_exec_result(&m_execResult);
  api::gvb_reset_exec_input(&m_execInput);

  if (m_vm) {
    api::gvb_vm_reset(m_vm);
  }
  if (m_device) {
    api::gvb_device_reset(m_device);
  }
}

void GvbSimWindow::reset(
    api::GvbVirtualMachine *vm, api::GvbDevice *device, const QString &name) {
  m_screen->setImageData(nullptr);
  if (m_vm) {
    api::gvb_destroy_vm(m_vm);
  }
  m_vm = vm;
  if (m_device) {
    api::gvb_destroy_device(m_device);
  }
  m_device = device;
  m_screen->setImageData(gvb_device_graphics_memory(device));
  m_name = name;
}

GvbSimWindow::~GvbSimWindow() {
  api::gvb_reset_exec_result(&m_execResult);
  api::gvb_reset_exec_input(&m_execInput);
  m_screen->setImageData(nullptr);
  if (m_vm) {
    api::gvb_destroy_vm(m_vm);
  }
  if (m_device) {
    api::gvb_destroy_device(m_device);
  }
}

void GvbSimWindow::initUi() {
  auto central = new QWidget;
  auto centralLayout = new QVBoxLayout(central);

  m_screen = new GvbSimScreen(this);
  m_screen->setContentsMargins(4, 4, 4, 4);
  centralLayout->addWidget(m_screen, 0, Qt::AlignHCenter);

  auto keyboard = new GvbSimKeyboard(central);
  keyboard->setContentsMargins(0, 4, 0, 0);
  centralLayout->addWidget(keyboard, 0, Qt::AlignHCenter);
  connect(keyboard, &GvbSimKeyboard::keyDown, this, &GvbSimWindow::keyDown);
  connect(keyboard, &GvbSimKeyboard::keyUp, this, &GvbSimWindow::keyUp);

  setCentralWidget(central);

  initToolBar();
}

void GvbSimWindow::initToolBar() {
  auto toolbar = addToolBar("");
  toolbar->setContextMenuPolicy(Qt::PreventContextMenu);
  toolbar->setMovable(false);

  auto actStart = toolbar->addAction("");
  actStart->setShortcut(Qt::Key_F5);
  connect(actStart, &QAction::triggered, this, [this] {
    m_editor->tryStartPause(this);
  });

  auto empty = new QWidget();
  empty->setMinimumWidth(20);
  toolbar->addWidget(empty);

  auto actStop =
      toolbar->addAction(QPixmap(":/assets/images/Stop.svg"), "停止");
  actStop->setShortcut(Qt::Key_F7);
  connect(actStop, &QAction::triggered, m_editor, &GvbEditor::stop);

  auto startIcon = QPixmap(":/assets/images/Run.svg");
  auto pauseIcon = QPixmap(":/assets/images/Pause.svg");

  connect(
      m_editor->m_stStarted, &QState::entered, this,
      [actStart, actStop, pauseIcon, this] {
        actStart->setText("暂停");
        actStart->setIcon(pauseIcon);
        actStop->setEnabled(true);
        m_state = "运行中";
        updateTitle();
      });
  connect(
      m_editor->m_stStopped, &QState::entered, this,
      [actStart, actStop, startIcon, this] {
        actStart->setText("运行");
        actStart->setIcon(startIcon);
        actStop->setEnabled(false);
        m_state = "运行结束";
        updateTitle();
      });
  connect(
      m_editor->m_stPaused, &QState::entered, this,
      [actStart, actStop, startIcon, this] {
        actStart->setText("继续");
        actStart->setIcon(startIcon);
        actStop->setEnabled(true);
        m_state = "已暂停";
        updateTitle();
      });
}

void GvbSimWindow::closeEvent(QCloseEvent *) {
  emit m_editor->stop();
}

void GvbSimWindow::start() {
  reset();
  execLater();
  startRepaintTimer();
}

void GvbSimWindow::cont() {
  m_paused = false;
  execLater();
}

void GvbSimWindow::pause() {
  m_paused = true;
}

void GvbSimWindow::stop() {
  stopCursorTimer();
  stopRepaintTimer();
  m_screen->update();

  if (m_execResult.tag == api::GvbExecResult::Tag::End) {
    return;
  }

  auto result = api::gvb_vm_stop(m_vm);
  if (result.tag == api::GvbStopVmResult::Tag::Left) {
    QMessageBox::critical(
        this, "错误",
        tr("运行时错误：%1")
            .arg(QString::fromUtf8(result.left._0.data, result.left._0.len)));
    destroy_string(result.left._0);
  }
  m_execResult.tag = api::GvbExecResult::Tag::End;
}

void GvbSimWindow::execLater() {
  QTimer::singleShot(0, this, [this] {
    if (m_paused) {
      return;
    }

    switch (m_execResult.tag) {
    case api::GvbExecResult::Tag::End:
      emit m_editor->stop();
      return;
    case api::GvbExecResult::Tag::Continue:
      break;
    case api::GvbExecResult::Tag::Sleep:
      sleep(m_execResult.sleep._0);
      return;
    case api::GvbExecResult::Tag::InKey: {
      if (!api::gvb_assign_device_key(m_device, &m_execInput)) {
        startCursorTimer();
        return;
      } else {
        stopCursorTimer();
      }
      break;
    }
    case api::GvbExecResult::Tag::KeyboardInput:
      startCursorTimer();
      // TODO
      break;
    case api::GvbExecResult::Tag::Error:
      printf(
          "%lu %s\n", m_execResult.error.location.line,
          QString::fromUtf8(
              m_execResult.error.message.data, m_execResult.error.message.len)
              .toStdString()
              .c_str());
      m_execResult.tag = api::GvbExecResult::Tag::End;
      emit m_editor->stop();
      // TODO show error message in editor
      return;
    }

    m_execResult = api::gvb_vm_exec(m_vm, m_execInput, EXEC_STEPS);
    api::gvb_reset_exec_input(&m_execInput);

    execLater();
  });
}

void GvbSimWindow::sleep(std::uint64_t ns) {
  QTimer::singleShot((ns + 500'000) / 1'000'000, this, [this] {
    m_execResult.tag = api::GvbExecResult::Tag::Continue;
    if (!m_paused) {
      execLater();
    }
  });
}

void GvbSimWindow::keyPressEvent(QKeyEvent *ev) {
  auto key = qtKeyToWqxKey(ev->key());
  if (key != 0) {
    keyDown(key);
  }
}

void GvbSimWindow::keyReleaseEvent(QKeyEvent *ev) {
  auto key = qtKeyToWqxKey(ev->key());
  if (key != 0) {
    keyUp(key);
  }
}

void GvbSimWindow::keyDown(std::uint8_t key) {
  api::gvb_device_fire_key_down(m_device, key);
  if (m_execResult.tag == api::GvbExecResult::Tag::InKey) {
    execLater();
  }
}

void GvbSimWindow::keyUp(std::uint8_t key) {
  api::gvb_device_fire_key_up(m_device, key);
}

void GvbSimWindow::timerEvent(QTimerEvent *ev) {
  if (ev->timerId() == m_timerCursor) {
    if (!m_paused) {
      api::gvb_device_blink_cursor(m_device);
    }
  } else if (ev->timerId() == m_timerRepaint) {
    auto dirty = api::gvb_device_screen_dirty_area(m_device);
    if (dirty.tag == api::Maybe<api::Rect>::Tag::Just) {
      auto left = static_cast<int>(dirty.just._0.left);
      auto top = static_cast<int>(dirty.just._0.top);
      auto right = static_cast<int>(dirty.just._0.right);
      auto bottom = static_cast<int>(dirty.just._0.bottom);
      m_screen->markDirty(QRect(QPoint(left, top), QPoint(right, bottom)));
      // TODO dirty area
      m_screen->update(
          QRect(QPoint(left * 2, top * 2), QPoint(right * 2, bottom * 2)));
    }
  }
}

void GvbSimWindow::startCursorTimer() {
  m_timerCursor = startTimer(500, Qt::PreciseTimer);
}

void GvbSimWindow::startRepaintTimer() {
  // 60fps
  m_timerRepaint = startTimer(17, Qt::PreciseTimer);
}

void GvbSimWindow::stopCursorTimer() {
  if (m_timerCursor) {
    killTimer(m_timerCursor);
    m_timerCursor = 0;
  }
}

void GvbSimWindow::stopRepaintTimer() {
  if (m_timerRepaint) {
    killTimer(m_timerRepaint);
    m_timerRepaint = 0;
  }
}

void GvbSimWindow::updateTitle() {
  setWindowTitle(tr("GVBASIC 模拟器 - %1 [%2]").arg(m_name).arg(m_state));
}