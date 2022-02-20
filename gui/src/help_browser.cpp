#include "help_browser.h"

#include <QHelpEngine>

HelpBrowser::HelpBrowser(QHelpEngine *helpEngine, QWidget *parent) :
  QTextBrowser(parent),
  m_helpEngine(helpEngine) {}

QVariant HelpBrowser::loadResource(int type, const QUrl &name) {
  if (name.scheme() == "qthelp") {
    return m_helpEngine->fileData(name);
  } else {
    return QTextBrowser::loadResource(type, name);
  }
}