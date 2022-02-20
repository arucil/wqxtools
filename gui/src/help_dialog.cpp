#include "help_dialog.h"

#include <QApplication>
#include <QDir>
#include <QFile>
#include <QHelpContentWidget>
#include <QHelpEngine>
#include <QHelpIndexWidget>
#include <QHelpLink>
#include <QHelpSearchEngine>
#include <QHelpSearchQueryWidget>
#include <QHelpSearchResultWidget>
#include <QSplitter>
#include <QTabWidget>
#include <QVBoxLayout>

#include "help_browser.h"

#define HELP_FILENAME "help.qhc"

HelpDialog::HelpDialog(QWidget *parent) : QDialog(parent) {
  QHelpEngine *helpEngine;
  if (QFile(HELP_FILENAME).exists()) {
    helpEngine = new QHelpEngine(HELP_FILENAME);
  } else {
    helpEngine = new QHelpEngine(
      QApplication::applicationDirPath() + QDir::separator() + HELP_FILENAME);
  }
  helpEngine->setupData();

  auto layout = new QVBoxLayout(this);

  auto splitter = new QSplitter(Qt::Horizontal, this);
  layout->addWidget(splitter);

  auto tab = new QTabWidget(splitter);
  tab->setMaximumWidth(200);
  tab->addTab(helpEngine->contentWidget(), "内容");

  auto search = new QWidget;
  tab->addTab(search, "搜索");

  auto searchLayout = new QVBoxLayout(search);
  auto searchEngine = helpEngine->searchEngine();
  auto searchQuery = searchEngine->queryWidget();
  auto searchResult = searchEngine->resultWidget();
  searchLayout->addWidget(searchQuery);
  searchLayout->addWidget(searchResult);
  connect(searchQuery, &QHelpSearchQueryWidget::search, [=] {
    searchEngine->search(searchQuery->searchInput());
  });

  auto textViewer = new HelpBrowser(helpEngine, splitter);
  textViewer->setSource(QUrl("qthelp://wqxtools/docs/index.html"));
  connect(
    helpEngine->contentWidget(),
    &QHelpContentWidget::linkActivated,
    textViewer,
    [=](const QUrl &name) { textViewer->setSource(name); });

  resize(800, 500);
}