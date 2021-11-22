#include "gvbsim_window.h"
#include "gvb.h"
#include "gvbeditor.h"
#include "gvbsim_screen.h"

GvbSimWindow::GvbSimWindow(GvbEditor *editor)
    : m_editor(editor), m_vm(nullptr), m_device(nullptr) {
  initUi();
}

void GvbSimWindow::reset() {
  if (m_vm) {
    gvb::vm_reset(m_vm);
  }
  if (m_device) {
    gvb::device_reset(m_device);
  }
}

void GvbSimWindow::reset(gvb::VirtualMachine *vm, gvb::Device *device) {
  if (m_vm) {
    gvb::destroy_vm(m_vm);
    m_vm = vm;
  }
  if (m_device) {
    gvb::destroy_device(m_device);
    m_device = device;
    m_screen->setImageData(gvb::device_graphics_memory(device));
  }
}

GvbSimWindow::~GvbSimWindow() {}

void GvbSimWindow::initUi() {
  m_screen = new GvbSimScreen(this);
  setCentralWidget(m_screen);
  adjustSize();
}