#pragma once

#include <QMainWindow>
#include <QString>
#include <cstdint>

#include "api.h"
#include "value.h"

class GvbEditor;
class GvbSimScreen;
class QCloseEvent;
class QKeyEvent;
class QTimerEvent;
class QShowEvent;

class GvbSimWindow: public QMainWindow {
  Q_OBJECT

public:
  GvbSimWindow(QWidget *, GvbEditor *);
  ~GvbSimWindow();

  void reset(api::GvbVirtualMachine *, api::GvbDevice *, const QString &);

protected:
  void closeEvent(QCloseEvent *) Q_DECL_OVERRIDE;
  void keyPressEvent(QKeyEvent *) Q_DECL_OVERRIDE;
  void keyReleaseEvent(QKeyEvent *) Q_DECL_OVERRIDE;
  void timerEvent(QTimerEvent *) Q_DECL_OVERRIDE;

private:
  void initUi();
  void initToolBar();
  void reset();
  void execLater();
  void sleep(std::uint64_t ns);
  void startCursorTimer();
  void startRepaintTimer();
  void stopCursorTimer();
  void stopRepaintTimer();

private slots:
  void start();
  void stop();
  void cont();
  void pause();
  void keyDown(std::uint8_t);
  void keyUp(std::uint8_t);
  void updateTitle();

private:
  GvbEditor *m_editor;
  api::GvbVirtualMachine *m_vm;
  api::GvbDevice *m_device;
  GvbSimScreen *m_screen;
  api::GvbExecResult m_execResult;
  api::GvbExecInput m_execInput;
  bool m_paused;
  int m_timerCursor;
  int m_timerRepaint;
  QString m_name;
  QString m_state;
  StrValue m_message;
};