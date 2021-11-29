#pragma once

#include <QDialog>
#include <QVector>

#include "api.h"

class GvbSimInputDialog: public QDialog {
  Q_OBJECT

public:
  GvbSimInputDialog(QWidget *, const api::GvbExecResult::KeyboardInput_Body &);

private:
  void initUi(const api::GvbExecResult::KeyboardInput_Body &);

private:
  QVector<api::GvbKeyboardInput> m_input;
};