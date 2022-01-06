#include "gvbsim_window.h"

#include <QHeaderView>
#include <QStatusBar>
#include <QKeyEvent>
#include <QMessageBox>
#include <QTableView>
#include <QTimer>
#include <QToolBar>
#include <QVBoxLayout>

#include "gvbeditor.h"
#include "gvbsim_input_dialog.h"
#include "gvbsim_keyboard.h"
#include "gvbsim_screen.h"

const size_t EXEC_STEPS = 50;

GvbSimWindow::GvbSimWindow(QWidget *parent, GvbEditor *editor) :
  QMainWindow(parent),
  m_editor(editor),
  m_vm(nullptr),
  m_device(nullptr),
  m_paused(false),
  m_timerCursor(0),
  m_timerRepaint(0),
  m_bindingModel(this) {
  initUi();

  connect(
    editor,
    &GvbEditor::start,
    this,
    &GvbSimWindow::start,
    Qt::QueuedConnection);
  connect(editor, &GvbEditor::stop, this, &GvbSimWindow::stop);
  connect(editor, &GvbEditor::cont, this, &GvbSimWindow::cont);
  connect(editor, &GvbEditor::pause, this, &GvbSimWindow::pause);

  m_execResult.tag = api::GvbExecResult::Tag::Continue;
  m_execInput.tag = api::GvbExecInput::Tag::None;

  adjustSize();

  QTimer::singleShot(0, this, [this] {
    m_message.setValue("点击工具栏的 [开始] 图标或按 [F5] 开始运行程序");
    centerWindow(this, qobject_cast<QMainWindow *>(this->parent())->screen());
  });
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
  api::GvbVirtualMachine *vm,
  api::GvbDevice *device,
  const QString &name) {
  m_screen->setImageData(nullptr);
  if (m_vm) {
    api::gvb_destroy_vm(m_vm);
  }
  m_vm = vm;
  m_bindingModel.setVm(vm);
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
    m_bindingModel.setVm(nullptr);
    api::gvb_destroy_vm(m_vm);
  }
  if (m_device) {
    api::gvb_destroy_device(m_device);
  }
}

void GvbSimWindow::initUi() {
  auto central = new QWidget;
  auto centralLayout = new QHBoxLayout(central);

  auto leftLayout = new QVBoxLayout();
  centralLayout->addLayout(leftLayout);

  initToolBar();
  leftLayout->addWidget(m_toolbar);

  m_screen = new GvbSimScreen(this);
  m_screen->setContentsMargins(4, 4, 4, 4);
  leftLayout->addWidget(m_screen, 0, Qt::AlignHCenter);

  auto keyboard = new GvbSimKeyboard(central);
  keyboard->setContentsMargins(0, 4, 0, 0);
  leftLayout->addWidget(keyboard, 0, Qt::AlignHCenter);
  connect(keyboard, &GvbSimKeyboard::keyDown, this, &GvbSimWindow::keyDown);
  connect(keyboard, &GvbSimKeyboard::keyUp, this, &GvbSimWindow::keyUp);

  m_bindingView = new QTableView();
  m_bindingView->resize(100, 0);
  m_bindingView->setModel(&m_bindingModel);
  m_bindingView->horizontalHeader()->setSectionResizeMode(QHeaderView::Stretch);
  m_bindingView->setItemDelegate(&m_bindingDelegate);
  connect(
    m_bindingView,
    &QTableView::doubleClicked,
    &m_bindingModel,
    &BindingModel::editValue);
  centralLayout->addWidget(m_bindingView);

  setCentralWidget(central);

  auto sb = new QStatusBar();
  connect(&m_message, &StrValue::changed, sb, [sb](const auto &msg) {
    sb->showMessage(msg);
  });
  leftLayout->addWidget(sb);

  connect(&m_state, &StrValue::changed, this, &GvbSimWindow::updateTitle);

  m_state.setValue("准备就绪");
}

void GvbSimWindow::initToolBar() {
  m_toolbar = new QToolBar();

  auto empty = new QWidget();
  empty->setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Expanding);
  m_toolbar->addWidget(empty);

  m_toolbar->setContextMenuPolicy(Qt::PreventContextMenu);
  m_toolbar->setMovable(false);

  m_actStart = m_toolbar->addAction("");
  m_actStart->setShortcut(Qt::Key_F5);
  connect(m_actStart, &QAction::triggered, this, [this] {
    m_editor->tryStartPause(this);
  });

  empty = new QWidget();
  empty->setMinimumWidth(30);
  m_toolbar->addWidget(empty);

  m_actStop = m_toolbar->addAction(QPixmap(":/images/Stop.svg"), "停止");
  m_actStop->setShortcut(Qt::Key_F7);
  connect(m_actStop, &QAction::triggered, m_editor, &GvbEditor::stop);

  empty = new QWidget();
  empty->setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Expanding);
  m_toolbar->addWidget(empty);

  auto startIcon = QPixmap(":/images/Run.svg");
  auto pauseIcon = QPixmap(":/images/Pause.svg");

  auto stoppedCallback = [startIcon, this] {
    m_actStart->setText("运行");
    m_actStart->setIcon(startIcon);
    m_actStop->setEnabled(false);
    m_state.setValue("运行结束");
  };

  connect(m_editor->m_stStarted, &QState::entered, this, [pauseIcon, this] {
    m_actStart->setText("暂停");
    m_actStart->setIcon(pauseIcon);
    m_actStop->setEnabled(true);
    m_state.setValue("运行中");
  });
  connect(m_editor->m_stStopped, &QState::entered, this, stoppedCallback);
  connect(m_editor->m_stPaused, &QState::entered, this, [startIcon, this] {
    m_actStart->setText("继续");
    m_actStart->setIcon(startIcon);
    m_actStop->setEnabled(true);
    m_state.setValue("已暂停");
  });

  stoppedCallback();
}

void GvbSimWindow::closeEvent(QCloseEvent *) {
  emit m_editor->stop();
}

void GvbSimWindow::setEnableBindingTable(bool enable) {
  m_bindingView->setEnabled(enable);
  if (enable) {
    m_bindingModel.enable();
    m_bindingView->setToolTip("");
  } else {
    m_bindingModel.disable();
    m_bindingView->setToolTip("暂停程序后才可以查看、修改变量");
  }
}

void GvbSimWindow::start() {
  reset();
  execLater();
  m_screen->update();
  startRepaintTimer();
  m_message.setValue("");
  setEnableBindingTable(false);
}

void GvbSimWindow::cont() {
  m_paused = false;
  execLater();
  setEnableBindingTable(false);
}

void GvbSimWindow::pause() {
  m_paused = true;
  setEnableBindingTable(true);
}

void GvbSimWindow::stop() {
  setEnableBindingTable(true);
  stopCursorTimer();
  stopRepaintTimer();

  if (m_execResult.tag == api::GvbExecResult::Tag::End) {
    m_screen->update();
    return;
  }

  auto result = api::gvb_vm_stop(m_vm);
  if (result.tag == api::GvbStopVmResult::Tag::Left) {
    QMessageBox::critical(
      this,
      "错误",
      QString("运行时错误：%1")
        .arg(QString::fromUtf8(result.left._0.data, result.left._0.len)));
    destroy_string(result.left._0);
  }
  api::gvb_reset_exec_result(&m_execResult);
  m_execResult.tag = api::GvbExecResult::Tag::End;
  m_screen->update();
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
      case api::GvbExecResult::Tag::KeyboardInput: {
        startCursorTimer();
        GvbSimInputDialog inputDlg(this, m_vm, m_execResult.keyboard_input);
        inputDlg.setModal(true);
        if (inputDlg.exec() == QDialog::Rejected) {
          emit m_editor->stop();
          return;
        }
        auto inputData = inputDlg.inputData();
        m_execInput.tag = api::GvbExecInput::Tag::KeyboardInput;
        m_execInput.keyboard_input._0 =
          api::gvb_new_input_array(inputData.constData(), inputData.size());
        stopCursorTimer();
        break;
      }
      case api::GvbExecResult::Tag::Error: {
        auto msg = QString::fromUtf8(
          m_execResult.error.message.data,
          m_execResult.error.message.len);
        m_execResult.tag = api::GvbExecResult::Tag::End;
        // TODO 程序运行时源码可能经过改动，报错的位置要更新
        m_message.setValue("程序运行出错，请在编辑器中查看错误信息");
        m_editor->showRuntimeError(m_execResult.error);
        emit m_editor->stop();
        return;
      }
    }

    api::gvb_reset_exec_result(&m_execResult);
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
      const auto scale = api::config()->gvb.simulator.pixel_scale;
      m_screen->update(QRect(
        QPoint(left * scale, top * scale),
        QPoint(right * scale, bottom * scale)));
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
  setWindowTitle(
    QString("GVBASIC 模拟器 - %1 [%2]").arg(m_name, m_state.value()));
}