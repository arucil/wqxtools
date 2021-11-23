#include "gvbsim_window.h"
#include "gvbeditor.h"
#include "gvbsim_screen.h"
#include <QDateTime>
#include <QHash>
#include <QKeyEvent>
#include <QMessageBox>
#include <QTimer>

const QHash<int, std::uint8_t> KEY_MAPPINGS{
    {Qt::Key_F1, 28},        {Qt::Key_F2, 29},       {Qt::Key_F3, 30},
    {Qt::Key_F4, 31},

    {Qt::Key_O, 111},        {Qt::Key_L, 108},       {Qt::Key_Up, 20},
    {Qt::Key_Down, 21},      {Qt::Key_P, 112},       {Qt::Key_Return, 13},
    {Qt::Key_PageDown, 14},  {Qt::Key_Right, 22},

    {Qt::Key_Q, 113},        {Qt::Key_W, 119},       {Qt::Key_E, 101},
    {Qt::Key_R, 114},        {Qt::Key_T, 116},       {Qt::Key_Y, 121},
    {Qt::Key_U, 117},        {Qt::Key_I, 105},

    {Qt::Key_A, 97},         {Qt::Key_S, 115},       {Qt::Key_D, 100},
    {Qt::Key_F, 102},        {Qt::Key_G, 103},       {Qt::Key_H, 104},
    {Qt::Key_J, 106},        {Qt::Key_K, 107},

    {Qt::Key_Z, 122},        {Qt::Key_X, 120},       {Qt::Key_C, 99},
    {Qt::Key_V, 118},        {Qt::Key_B, 98},        {Qt::Key_N, 110},
    {Qt::Key_M, 109},        {Qt::Key_PageUp, 19},

    {Qt::Key_Control, 25}, // [Ctrl] -> [求助]
    {Qt::Key_Shift, 26},     {Qt::Key_CapsLock, 18}, {Qt::Key_Escape, 27},
    {Qt::Key_0, 48},         {Qt::Key_Period, 46},   {Qt::Key_Space, 32},
    {Qt::Key_Left, 23},

    {Qt::Key_1, 98},         {Qt::Key_2, 110},       {Qt::Key_3, 109},
    {Qt::Key_4, 103},        {Qt::Key_5, 104},       {Qt::Key_6, 106},
    {Qt::Key_7, 121},        {Qt::Key_8, 117},       {Qt::Key_9, 105},

    {Qt::Key_Enter, 13}, // Numpad Enter

    {Qt::Key_AsciiTilde, 25} // [~] -> [求助]
};

const size_t EXEC_STEPS = 50;

GvbSimWindow::GvbSimWindow(QWidget *parent, GvbEditor *editor)
    : QMainWindow(parent), m_editor(editor), m_vm(nullptr), m_device(nullptr),
      m_paused(false), m_lastPaintTime(0), m_timerCursor(0) {
  initUi();

  connect(editor, &GvbEditor::start, this, &GvbSimWindow::start);
  connect(editor, &GvbEditor::stop, this, &GvbSimWindow::stop);
  connect(editor, &GvbEditor::cont, this, &GvbSimWindow::cont);
  connect(editor, &GvbEditor::pause, this, &GvbSimWindow::pause);

  m_execResult.tag = gvb::ExecResult::Tag::Continue;
  m_execInput.tag = gvb::ExecInput::Tag::None;

  adjustSize();
}

void GvbSimWindow::reset() {
  m_paused = false;
  gvb::reset_exec_result(&m_execResult);
  gvb::reset_exec_input(&m_execInput);

  if (m_vm) {
    gvb::vm_reset(m_vm);
  }
  if (m_device) {
    gvb::device_reset(m_device);
  }
}

void GvbSimWindow::reset(gvb::VirtualMachine *vm, gvb::Device *device) {
  m_screen->setImageData(nullptr);
  if (m_vm) {
    gvb::destroy_vm(m_vm);
  }
  m_vm = vm;
  if (m_device) {
    gvb::destroy_device(m_device);
  }
  m_device = device;
  m_screen->setImageData(gvb::device_graphics_memory(device));
}

GvbSimWindow::~GvbSimWindow() {
  gvb::reset_exec_result(&m_execResult);
  gvb::reset_exec_input(&m_execInput);
  m_screen->setImageData(nullptr);
  if (m_vm) {
    gvb::destroy_vm(m_vm);
  }
  if (m_device) {
    gvb::destroy_device(m_device);
  }
}

void GvbSimWindow::initUi() {
  m_screen = new GvbSimScreen(this);
  setCentralWidget(m_screen);
}

void GvbSimWindow::closeEvent(QCloseEvent *) {
  emit m_editor->stop();
}

void GvbSimWindow::start() {
  reset();
  execLater();
}

void GvbSimWindow::cont() {
  m_paused = false;
  execLater();
}

void GvbSimWindow::pause() {
  m_paused = true;
}

void GvbSimWindow::stop() {
  if (m_execResult.tag == gvb::ExecResult::Tag::End) {
    return;
  }

  if (m_timerCursor) {
    killTimer(m_timerCursor);
    m_timerCursor = 0;
  }

  auto result = gvb::vm_stop(m_vm);
  if (result.tag == gvb::StopVmResult::Tag::Left) {
    QMessageBox::critical(
        this, "错误",
        tr("运行时错误：%1")
            .arg(QString::fromUtf8(result.left._0.data, result.left._0.len)));
    gvb::destroy_string(result.left._0);
  }
  m_execResult.tag = gvb::ExecResult::Tag::End;
}

void GvbSimWindow::execLater() {
  QTimer::singleShot(0, this, [this] {
    switch (m_execResult.tag) {
    case gvb::ExecResult::Tag::End:
      return;
    case gvb::ExecResult::Tag::Continue:
      break;
    case gvb::ExecResult::Tag::Sleep:
      sleep(m_execResult.sleep._0);
      return;
    case gvb::ExecResult::Tag::InKey: {
      if (!gvb::assign_device_key(m_device, &m_execInput)) {
        m_timerCursor = startTimer(500, Qt::PreciseTimer);
        return;
      } else {
        if (m_timerCursor) {
          killTimer(m_timerCursor);
          m_timerCursor = 0;
        }
      }
      break;
    }
    case gvb::ExecResult::Tag::KeyboardInput:
      // TODO
      break;
    case gvb::ExecResult::Tag::Error:
      printf(
          "%lu %s\n", m_execResult.error.location.line,
          QString::fromUtf8(
              m_execResult.error.message.data, m_execResult.error.message.len)
              .toStdString()
              .c_str());
      m_execResult.tag = gvb::ExecResult::Tag::End;
      emit m_editor->stop();
      // TODO show error message in editor
      return;
    }

    m_execResult = gvb::vm_exec(m_vm, m_execInput, EXEC_STEPS);
    gvb::reset_exec_input(&m_execInput);

    auto time = QDateTime::currentMSecsSinceEpoch();
    // 50fps
    if (time - m_lastPaintTime >= 20) {
      auto dirty = gvb::device_screen_dirty_area(m_device);
      if (dirty.tag == gvb::Maybe<gvb::Rect>::Tag::Just) {
        auto left = static_cast<int>(dirty.just._0.left);
        auto top = static_cast<int>(dirty.just._0.top);
        auto right = static_cast<int>(dirty.just._0.right);
        auto bottom = static_cast<int>(dirty.just._0.bottom);
        m_screen->markDirty(QRect(QPoint(left, top), QPoint(right, bottom)));
        // TODO dirty area
        m_screen->update(QRect(QPoint(left * 2, top * 2), QPoint(right * 2, bottom * 2)));
        m_lastPaintTime = time;
      }
    }

    if (!m_paused) {
      execLater();
    }
  });
}

void GvbSimWindow::sleep(std::uint64_t ns) {
  QTimer::singleShot((ns + 500'000) / 1'000'000, this, [this] {
    m_execResult.tag = gvb::ExecResult::Tag::Continue;
    if (!m_paused) {
      execLater();
    }
  });
}

void GvbSimWindow::keyPressEvent(QKeyEvent *ev) {
  auto it = KEY_MAPPINGS.constFind(ev->key());
  if (it != KEY_MAPPINGS.constEnd()) {
    gvb::device_fire_key_down(m_device, *it);
    if (m_execResult.tag == gvb::ExecResult::Tag::InKey) {
      execLater();
    }
  }
}

void GvbSimWindow::keyReleaseEvent(QKeyEvent *ev) {
  auto it = KEY_MAPPINGS.constFind(ev->key());
  if (it != KEY_MAPPINGS.constEnd()) {
    gvb::device_fire_key_up(m_device, *it);
  }
}

void GvbSimWindow::timerEvent(QTimerEvent *ev) {
  if (ev->timerId() == m_timerCursor) {
    if (!m_paused) {
      gvb::device_blink_cursor(m_device);
    }
  }
}