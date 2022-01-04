#pragma once

#include <QMainWindow>
#include <variant>

class QScreen;

QMainWindow *getMainWindow();

QString getSystemDir(const char *name);

void centerWindow(QMainWindow *, QScreen *);

enum class ActionResult {
  Fail,
  Succeed,
};

using Unit = std::monostate;
