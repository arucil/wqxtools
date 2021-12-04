#include "util.h"

#include <QApplication>
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