#pragma once

#include <QMainWindow>
#include <variant>

QMainWindow *getMainWindow();

enum class ActionResult {
  Fail,
  Succeed,
};

using Unit = std::monostate;
