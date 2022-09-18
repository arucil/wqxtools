#include "util.h"

#include <QApplication>
#include <QDebug>
#include <QDir>
#include <QScreen>

QMainWindow *getMainWindow() {
  foreach (QWidget *widget, qApp->topLevelWidgets())
    if (QMainWindow *mainWindow = qobject_cast<QMainWindow *>(widget))
      return mainWindow;
  return NULL;
}

void centerWindow(QMainWindow *window, QScreen *screen) {
  auto size = window->frameGeometry().size();
  window->move(
    screen->geometry().center() - QPoint(size.width() / 2, size.height() / 2));
}

QString getSystemDir(const char *name) {
  QDir workDir(QDir::currentPath());
  if (workDir.cd(name)) {
    return workDir.absolutePath();
  }
  auto path = QDir::cleanPath(QCoreApplication::applicationDirPath());
  QDir dir(path);
  if (!dir.cd(name)) {
    dir.mkdir(name);
    dir.cd(name);
  }
  return dir.absolutePath();
}