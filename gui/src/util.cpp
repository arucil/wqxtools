#include <QApplication>
#include "util.h"

QMainWindow *getMainWindow() {
  foreach (QWidget *widget, qApp->topLevelWidgets())
    if (QMainWindow *mainWindow = qobject_cast<QMainWindow *>(widget))
      return mainWindow;
  return NULL;
}