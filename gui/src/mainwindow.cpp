#include "mainwindow.h"
#include "gvbeditor.h"
#include "tool_factory.h"
#include "value.h"
#include <QApplication>
#include <QCloseEvent>
#include <QFileDialog>
#include <QFileInfo>
#include <QMenu>
#include <QMenuBar>
#include <QMessageBox>
#include <QSplitter>
#include <QTimer>

MainWindow::MainWindow(QWidget *parent) : QMainWindow(parent) {
  initUi();

  resize(800, 540);
  setWindowTitle("WQX工具箱");
}

void MainWindow::initUi() {
  initMenu();
}

void MainWindow::initMenu() {
  auto mnuFile = menuBar()->addMenu("文件(&F)");

  auto actOpen = mnuFile->addAction("打开(&O)");
  actOpen->setShortcut(Qt::CTRL | Qt::Key_O);
  connect(actOpen, &QAction::triggered, this, &MainWindow::openFile);

  auto actNew = mnuFile->addAction("新建(&N)");
  actNew->setShortcut(Qt::CTRL | Qt::Key_N);
  connect(actNew, &QAction::triggered, this, &MainWindow::createFile);

  mnuFile->addSeparator();

  m_actSave = mnuFile->addAction("保存(&S)");
  m_actSave->setShortcut(Qt::CTRL | Qt::Key_S);
  connect(m_actSave, &QAction::triggered, this, &MainWindow::saveFile);

  m_actSaveAs = mnuFile->addAction("另存为...");
  connect(m_actSaveAs, &QAction::triggered, this, &MainWindow::saveFileAs);

  mnuFile->addSeparator();

  m_actExit = mnuFile->addAction("退出");
  m_actExit->setShortcut(Qt::ALT | Qt::Key_F4);
  connect(m_actExit, &QAction::triggered, qApp, &QApplication::quit);

  auto mnuEdit = menuBar()->addMenu("编辑(&E)");

  m_actUndo = mnuEdit->addAction("撤销");
  m_actUndo->setShortcut(Qt::CTRL | Qt::Key_Z);

  m_actRedo = mnuEdit->addAction("重做");
  m_actRedo->setShortcut(Qt::CTRL | Qt::Key_Y);

  mnuEdit->addSeparator();

  m_actCopy = mnuEdit->addAction("复制");
  m_actCopy->setShortcut(Qt::CTRL | Qt::Key_C);

  m_actCut = mnuEdit->addAction("剪切");
  m_actCut->setShortcut(Qt::CTRL | Qt::Key_X);

  m_actPaste = mnuEdit->addAction("粘贴");
  m_actPaste->setShortcut(Qt::CTRL | Qt::Key_V);

  mnuEdit->addSeparator();

  m_actFind = mnuEdit->addAction("查找");
  m_actFind->setShortcut(Qt::CTRL | Qt::Key_F);

  m_actReplace = mnuEdit->addAction("替换");
  m_actReplace->setShortcut(Qt::CTRL | Qt::Key_H);
}

void MainWindow::closeEvent(QCloseEvent *event) {
  if (0) {
    event->ignore();
  }
}

void MainWindow::openFile() {
  QString filter;
  auto semi = false;
  for (const auto &i : ToolFactoryRegistry::getExtensions()) {
    if (semi) {
      filter += ";;";
    }
    semi = true;
    filter += i.first;
    filter += " (";
    for (auto &ext : i.second) {
      filter += "*.";
      filter += ext;
      filter += " ";
    }
    filter += ")";
  }
  auto path = QFileDialog::getOpenFileName(
      this, "", "", filter, nullptr,
      QFileDialog::Option::DontResolveSymlinks |
          QFileDialog::Option::DontUseNativeDialog);
  openFileByPath(path);
}

void MainWindow::openFileByPath(const QString &path) {
  if (path.isEmpty()) {
    return;
  }

  // TODO confirm if m_doc is dirty
  auto old = centralWidget();
  if (auto oldWidget = dynamic_cast<EditCapabilities *>(old)) {
    if (oldWidget->m_dirty {
    }
  }

  auto ext = QFileInfo(path).suffix();
  if (ext.isEmpty()) {
    QMessageBox::critical(
        this, "文件打开失败", "文件缺少后缀名，无法识别文件类型");
    return;
  }

  auto ctor = ToolFactoryRegistry::get(ext.toLower());

  if (ctor.has_value()) {
    auto widget = ctor.value()(this);
    setCentralWidget(widget);
    QTimer::singleShot(0, [widget, path] {
      widget->load(path);
    });

    auto canSave = dynamic_cast<FileCapabilities *>(widget) != nullptr;
    m_actSave->setEnabled(canSave);
    m_actSaveAs->setEnabled(canSave);

    if (auto editor = dynamic_cast<EditCapabilities *>(widget)) {
      connect(
          &editor->m_copyCutEnabled, &BoolValue::changed, m_actCopy,
          &QAction::setEnabled);
      connect(
          &editor->m_copyCutEnabled, &BoolValue::changed, m_actCut,
          &QAction::setEnabled);
      connect(
          &editor->m_pasteEnabled, &BoolValue::changed, m_actPaste,
          &QAction::setEnabled);
      connect(
          &editor->m_undoEnabled, &BoolValue::changed, m_actUndo,
          &QAction::setEnabled);
      connect(
          &editor->m_redoEnabled, &BoolValue::changed, m_actRedo,
          &QAction::setEnabled);
      connect(m_actCopy, &QAction::triggered, [editor] {
        editor->copy();
      });
      connect(m_actCut, &QAction::triggered, [editor] {
        editor->cut();
      });
      connect(m_actPaste, &QAction::triggered, [editor] {
        editor->paste();
      });
      connect(m_actUndo, &QAction::triggered, [editor] {
        editor->undo();
      });
      connect(m_actRedo, &QAction::triggered, [editor] {
        editor->redo();
      });
      connect(m_actFind, &QAction::triggered, [editor] {
        editor->find();
      });
      connect(m_actReplace, &QAction::triggered, [editor] {
        editor->replace();
      });
    }
  } else {
    QMessageBox::critical(this, "文件打开失败", "无法识别该文件类型");
  }
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