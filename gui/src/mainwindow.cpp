#include "mainwindow.h"
#include <QApplication>
#include <QGridLayout>
#include <QIcon>
#include <QLabel>
#include <QMenu>
#include <QMenuBar>
#include <QPushButton>
#include <QStatusBar>
#include <QString>
#include <QToolBar>
#include <QCloseEvent>
#include <ScintillaEdit.h>

MainWindow::MainWindow(QWidget *parent) : QMainWindow(parent) {
  setupUi();

  auto a = new QAction("&Fuck", this);
  statusBar()->addAction(a);
  statusBar()->showMessage("shit????");

  auto central = new QWidget;

  auto btn1 = new QPushButton("plus", central);
  auto btn2 = new QPushButton("minus", central);
  auto label = new QLabel(central);

  auto layout = new QGridLayout(central);
  layout->addWidget(btn1, 0, 0);
  layout->addWidget(btn2, 0, 1);
  layout->addWidget(label, 1, 0, 1, 2);

  auto edit = new ScintillaEdit(this);
  edit->styleSetFont(STYLE_DEFAULT, "WenQuXing");
  edit->styleSetSize(STYLE_DEFAULT, 14);
  auto font = QFont("WenQuXing");
  font.setPointSize(14);
  font.setStyleStrategy(QFont::StyleStrategy::PreferOutline);
  layout->addWidget(edit, 2, 0, 1, 2);

  setCentralWidget(central);

  auto toolbar = addToolBar("default");
  toolbar->addAction("hhh")->setIcon(QIcon(":/assets/images/ferris.png"));

  resize(600, 400);
  setWindowTitle("WQX工具箱");
}

void MainWindow::setupUi() { setupMenu(); }

void MainWindow::setupMenu() {
  auto mnuFile = menuBar()->addMenu("文件(&F)");

  auto mnuOpen = mnuFile->addAction("打开(&O)");
  mnuOpen->setShortcut(Qt::CTRL | Qt::Key_O);
  connect(mnuOpen, &QAction::triggered, this, &MainWindow::openFile);

  auto mnuNew = mnuFile->addAction("新建(&N)");
  mnuNew->setShortcut(Qt::CTRL | Qt::Key_N);
  connect(mnuNew, &QAction::triggered, this, &MainWindow::createFile);

  mnuFile->addSeparator();

  m_mnuSave = mnuFile->addAction("保存(&S)");
  m_mnuSave->setShortcut(Qt::CTRL | Qt::Key_S);
  connect(m_mnuSave, &QAction::triggered, this, &MainWindow::saveFile);

  m_mnuSaveAs = mnuFile->addAction("另存为...");
  connect(m_mnuSaveAs, &QAction::triggered, this, &MainWindow::saveFileAs);

  mnuFile->addSeparator();

  m_mnuExit = mnuFile->addAction("退出");
  m_mnuExit->setShortcut(Qt::ALT | Qt::Key_F4);
  connect(m_mnuExit, &QAction::triggered, qApp, &QApplication::quit);
}

void MainWindow::closeEvent(QCloseEvent *event) {
    if (0) {
        event->ignore();
    }
}

void MainWindow::openFile() {
    // TODO
}

void MainWindow::createFile() {
    // TODO
}

void MainWindow::saveFile() {
    // TODO
}

void MainWindow::saveFileAs() {
    // TODO
}