#pragma once

#include <QMainWindow>

class GvbEditor;
class GvbSimScreen;

namespace gvb {
  class VirtualMachine;
  class Device;
}

class GvbSimWindow : public QMainWindow {
  Q_OBJECT

public:
  GvbSimWindow(GvbEditor *);
  ~GvbSimWindow();

  void reset(gvb::VirtualMachine *, gvb::Device *);
  void reset();

private:
  void initUi();

private:
  GvbEditor *m_editor;
  gvb::VirtualMachine *m_vm;
  gvb::Device *m_device;
  GvbSimScreen *m_screen;
};