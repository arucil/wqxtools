#include <QApplication>
#include <QFontDatabase>
#include <QVector>
#include <QWidget>

#include "mainwindow.h"
#include "tool_registry.h"

void loadResources();
void initTools();

int main(int argc, char *argv[]) {
  QApplication app(argc, argv);

  uint16_t shit[] = {0xd800};
  QString::fromUtf16(shit, 1);

  loadResources();
  initTools();

  if (MainWindow::loadConfig(nullptr) == ActionResult::Fail) {
    return 1;
  }

  MainWindow window;
  window.show();

  return app.exec();
}

void loadResources() {
  QFontDatabase::addApplicationFont(":/fonts/WenQuXing.ttf");
}

void initTools() {
  ToolRegistry::registerTool(
    "GVBASIC文件",
    {{"bas", "txt"},
     [](QWidget *parent) -> ToolWidget * { return new GvbEditor(parent); },
     [](ToolWidget *widget) {
       return qobject_cast<GvbEditor *>(widget) != nullptr;
     },
     true});
}
