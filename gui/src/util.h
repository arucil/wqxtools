#pragma once

#include <QMainWindow>

QMainWindow *getMainWindow();

enum class ActionResult {
  Fail,
  Succeed,
};