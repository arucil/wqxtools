#include "gvbeditor.h"
#include "mainwindow.h"
#include "tool_factory.h"
#include "gvb.h"
#include <QApplication>
#include <QFontDatabase>
#include <QWidget>
#include <vector>

void loadResources();
void initTools();

int main(int argc, char *argv[]) {
  QApplication app(argc, argv);

  loadResources();
  initTools();

  qRegisterMetaType<std::vector<Diagnostic>>();

  MainWindow window;
  window.show();

  return app.exec();
}

void loadResources() {
  QFontDatabase::addApplicationFont(":/assets/fonts/WenQuXing.ttf");
}

void initTools() {
  ToolFactory bas = {
      {"bas", "txt"}, [](auto parent) { return new GvbEditor(parent); }};
  ToolFactoryRegistry::registerFactory("GVBASIC文件", bas);
}