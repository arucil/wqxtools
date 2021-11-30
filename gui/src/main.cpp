#include <QApplication>
#include <QFontDatabase>
#include <QWidget>
#include <vector>

#include "gvbeditor.h"
#include "mainwindow.h"
#include "tool_factory.h"

void loadResources();
void initTools();

int main(int argc, char *argv[]) {
  QApplication app(argc, argv);

  uint16_t shit[]={0xd800};
  QString::fromUtf16(shit,1);

  loadResources();
  initTools();

  if (MainWindow::loadConfig(nullptr) == ActionResult::Fail) {
    return 1;
  }

  qRegisterMetaType<std::vector<Diagnostic>>();

  MainWindow window;
  window.show();

  return app.exec();
}

void loadResources() {
  QFontDatabase::addApplicationFont(":/assets/fonts/WenQuXing.ttf");
}

void initTools() {
  ToolFactory bas = {{"bas", "txt"}, [](auto parent) {
                       return new GvbEditor(parent);
                     }};
  ToolFactoryRegistry::registerFactory("GVBASIC文件", bas);
}
