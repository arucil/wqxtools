#pragma once

#include <QMainWindow>
#include <cstdint>
#include "gvb.h"

class GvbEditor;
class GvbSimScreen;
class QCloseEvent;
class QKeyEvent;
class QTimerEvent;

class GvbSimWindow : public QMainWindow {
  Q_OBJECT

public:
  GvbSimWindow(QWidget *, GvbEditor *);
  ~GvbSimWindow();

  void reset(gvb::VirtualMachine *, gvb::Device *);

protected:
  void closeEvent(QCloseEvent *) Q_DECL_OVERRIDE;
  void keyPressEvent(QKeyEvent *) Q_DECL_OVERRIDE;
  void keyReleaseEvent(QKeyEvent *) Q_DECL_OVERRIDE;
  void timerEvent(QTimerEvent *) Q_DECL_OVERRIDE;

private:
  void initUi();
  void reset();
  void execLater();
  void sleep(std::uint64_t ns);

private slots:
  void start();
  void stop();
  void cont();
  void pause();

private:
  GvbEditor *m_editor;
  gvb::VirtualMachine *m_vm;
  gvb::Device *m_device;
  GvbSimScreen *m_screen;
  gvb::ExecResult m_execResult;
  gvb::ExecInput m_execInput;
  bool m_paused;
  std::int64_t m_lastPaintTime;
  int m_timerCursor;
};