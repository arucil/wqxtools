#include "mainwindow.h"
#include <QApplication>
#include <QFontDatabase>

void loadResources();

int main(int argc, char *argv[]) {
  QApplication app(argc, argv);

  loadResources();

  MainWindow window;
  window.show();

  return app.exec();
}

void loadResources() {
  QFontDatabase::addApplicationFont(":/assets/fonts/WenQuXing.ttf");
}