#include "mainwindow.h"
#include "action.h"
#include "gvb.h"
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

#define WINDOW_TITLE "WQX 工具箱"

MainWindow::MainWindow(QWidget *parent) : QMainWindow(parent) {
  initUi();

  resize(800, 540);

  QTimer::singleShot(0, [this] {
    m_loaded.setValue(false);

    m_actSave->setEnabled(false);
    m_actSaveAs->setEnabled(false);
    m_actUndo->setEnabled(false);
    m_actRedo->setEnabled(false);
    m_actCopy->setEnabled(false);
    m_actCut->setEnabled(false);
    m_actPaste->setEnabled(false);
    m_actFind->setEnabled(false);
    m_actReplace->setEnabled(false);
    m_actStart->setEnabled(false);
    m_actStop->setEnabled(false);
  });
}

void MainWindow::initUi() {
  initMenu();

  connect(&m_openFilePath, &StrValue::changed, this, &MainWindow::setTitle);
  setTitle();
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
  connect(&m_loaded, &BoolValue::changed, m_actSave, &QAction::setEnabled);

  m_actSaveAs = mnuFile->addAction("另存为...");
  connect(m_actSaveAs, &QAction::triggered, this, &MainWindow::saveFileAs);
  connect(&m_loaded, &BoolValue::changed, m_actSaveAs, &QAction::setEnabled);

  mnuFile->addSeparator();

  auto actExit = mnuFile->addAction("退出");
  actExit->setShortcut(Qt::ALT | Qt::Key_F4);
  connect(actExit, &QAction::triggered, qApp, &QApplication::quit);

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

  auto mnuProg = menuBar()->addMenu("程序(&P)");

  m_actStart = mnuProg->addAction("运行");
  m_actStart->setShortcut(Qt::Key_F5);

  m_actStop = mnuProg->addAction("停止");
  m_actStop->setShortcut(Qt::Key_F7);

  mnuProg->addSeparator();

  auto actConfig = mnuProg->addAction("重新加载配置文件");
  connect(actConfig, &QAction::triggered, this, [this] {
    loadConfig(this);
  });
}

void MainWindow::closeEvent(QCloseEvent *event) {
  auto widget = static_cast<Tool *>(centralWidget());
  if (confirmSaveIfDirty(widget) == ActionResult::Fail) {
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

  auto widget = static_cast<Tool *>(centralWidget());
  if (confirmSaveIfDirty(widget) == ActionResult::Fail) {
    return;
  }

  auto fileinfo = QFileInfo(path);
  auto ext = fileinfo.suffix();
  if (ext.isEmpty()) {
    QMessageBox::critical(
        this, "文件打开失败", "文件缺少后缀名，无法识别文件类型");
    return;
  }

  if (!widget || !widget->canLoad(path)) {
    auto ctor = ToolFactoryRegistry::get(ext.toLower());
    if (!ctor.has_value()) {
      QMessageBox::critical(this, "文件打开失败", "无法识别该文件类型");
      return;
    }

    widget = ctor.value()(this);
    setCentralWidget(widget);
  }

  QTimer::singleShot(0, [widget, path, this] {
    auto result = widget->load(path);
    if (auto err = std::get_if<QString>(&result)) {
      QMessageBox::critical(this, "文件打开失败", *err);
      setCentralWidget(nullptr);
      m_openFilePath.setValue(QString());
      m_loaded.setValue(false);
    } else {
      m_loaded.setValue(true);
    }
  });

  m_openFilePath.setValue(fileinfo.fileName());

  auto fileCap = dynamic_cast<FileCapabilities *>(widget);
  m_actSave->setEnabled(fileCap != nullptr);
  m_actSaveAs->setEnabled(fileCap != nullptr);

  if (auto editor = dynamic_cast<EditCapabilities *>(widget)) {
    m_actCopy->setEnabled(editor->m_actCopy);
    connect(
        editor->m_actCopy, &Action::enabledChanged, m_actCopy,
        &QAction::setEnabled);
    m_actCut->setEnabled(editor->m_actCut);
    connect(
        editor->m_actCut, &Action::enabledChanged, m_actCut,
        &QAction::setEnabled);
    m_actPaste->setEnabled(editor->m_actPaste);
    connect(
        editor->m_actPaste, &Action::enabledChanged, m_actPaste,
        &QAction::setEnabled);
    m_actUndo->setEnabled(editor->m_actUndo);
    connect(
        editor->m_actUndo, &Action::enabledChanged, m_actUndo,
        &QAction::setEnabled);
    m_actRedo->setEnabled(editor->m_actRedo);
    connect(
        editor->m_actRedo, &Action::enabledChanged, m_actRedo,
        &QAction::setEnabled);
    m_actFind->setEnabled(true);
    m_actReplace->setEnabled(true);
    connect(
        m_actCopy, &QAction::triggered, editor->m_actCopy, &QAction::trigger);
    connect(m_actCut, &QAction::triggered, editor->m_actCut, &QAction::trigger);
    connect(
        m_actPaste, &QAction::triggered, editor->m_actPaste, &QAction::trigger);
    connect(
        m_actUndo, &QAction::triggered, editor->m_actUndo, &QAction::trigger);
    connect(
        m_actRedo, &QAction::triggered, editor->m_actRedo, &QAction::trigger);
    connect(
        m_actFind, &QAction::triggered, editor->m_actFind, &QAction::trigger);
    connect(
        m_actReplace, &QAction::triggered, editor->m_actReplace,
        &QAction::trigger);

    connect(&editor->m_dirty, &BoolValue::changed, this, &MainWindow::setTitle);
  } else {
    m_actCopy->setEnabled(false);
    m_actCut->setEnabled(false);
    m_actPaste->setEnabled(false);
    m_actUndo->setEnabled(false);
    m_actRedo->setEnabled(false);
    m_actFind->setEnabled(false);
    m_actReplace->setEnabled(false);
  }

  if (auto progCap = dynamic_cast<ProgramCapabilities *>(widget)) {
    m_actStart->setEnabled(progCap->m_actStart->isEnabled());
    connect(
        progCap->m_actStart, &Action::enabledChanged, m_actStart,
        &QAction::setEnabled);
    m_actStop->setEnabled(progCap->m_actStop->isEnabled());
    connect(
        progCap->m_actStop, &Action::enabledChanged, m_actStop,
        &QAction::setEnabled);
    connect(
        progCap->m_actStop, &Action::enabledChanged, this,
        &MainWindow::setStartButtonText);
    connect(
        &progCap->m_isPaused, &BoolValue::changed, this,
        &MainWindow::setStartButtonText);
  } else {
    m_actStart->setEnabled(false);
    m_actStop->setEnabled(false);
    setStartButtonText();
  }

  if (fileCap && fileCap->m_actSave != nullptr) {
    connect(
        fileCap->m_actSave, &QAction::triggered, this, &MainWindow::saveFile);
  }
}

void MainWindow::setStartButtonText() {
  if (auto progCap = dynamic_cast<ProgramCapabilities *>(centralWidget())) {
    m_actStart->setText(
        progCap->m_isPaused.value()
            ? "继续"
            : progCap->m_actStop->isEnabled() ? "暂停" : "运行");
  } else {
    m_actStart->setText("运行");
  }
}

ActionResult MainWindow::confirmSaveIfDirty(Tool *widget) {
  if (auto oldWidget = dynamic_cast<EditCapabilities *>(widget)) {
    if (oldWidget->m_dirty.value()) {
      auto btn = QMessageBox::question(
          this, "文件改动",
          tr("文件 %1 有改动，是否保存？")
              .arg(QFileInfo(m_openFilePath.value()).fileName()),
          QMessageBox::StandardButton::Yes | QMessageBox::StandardButton::No |
              QMessageBox::StandardButton::Cancel);
      switch (btn) {
      case QMessageBox::StandardButton::Yes:
        return saveFile();
      case QMessageBox::StandardButton::No:
        return ActionResult::Succeed;
      default:
        return ActionResult::Fail;
      }
    }
  }
  return ActionResult::Succeed;
}

void MainWindow::createFile() {
  // TODO
}

ActionResult MainWindow::saveFile() {
  if (m_openFilePath.value().isEmpty()) {
    return saveFileAs(true);
  }
  if (auto edit = dynamic_cast<FileCapabilities *>(centralWidget())) {
    return handleSaveFileError(edit->save(m_openFilePath.value()));
  }
  return ActionResult::Succeed;
}

ActionResult MainWindow::saveFileAs(bool save) {
  if (auto edit = dynamic_cast<FileCapabilities *>(centralWidget())) {
    auto ext = QFileInfo(m_openFilePath.value()).suffix().toLower();
    QString filter;
    auto semi = false;
    for (const auto &i : ToolFactoryRegistry::getExtensions()) {
      if (i.second.count(ext)) {
        for (const auto &ext : i.second) {
          if (semi) {
            filter += ";;";
          }
          semi = true;
          filter += i.first;
          filter += " (";
          filter += " *.";
          filter += ext;
          filter += ")";
        }
      }
    }
    auto path = QFileDialog::getSaveFileName(
        this, save ? "保存文件" : "另存为", "", filter, nullptr,
        QFileDialog::Option::DontResolveSymlinks |
            QFileDialog::Option::DontUseNativeDialog);
    if (path.isEmpty()) {
      return ActionResult::Fail;
    }

    if (QFileInfo(path).suffix().isEmpty() &&
        !m_openFilePath.value().isEmpty()) {
      auto ext = QFileInfo(m_openFilePath.value()).suffix();
      path += '.';
      path += ext;
    }

    auto f = QFileInfo(path);
    if (f.exists()) {
      auto res = QMessageBox::question(
          this, save ? "保存文件" : "另存为",
          tr("文件 %1 已存在，是否覆盖？").arg(f.fileName()),
          QMessageBox::StandardButton::Yes | QMessageBox::StandardButton::No |
              QMessageBox::StandardButton::Cancel);
      if (res == QMessageBox::StandardButton::Cancel) {
        return ActionResult::Fail;
      } else if (res == QMessageBox::StandardButton::No) {
        return ActionResult::Succeed;
      }
    }

    m_openFilePath.setValue(path);
    return handleSaveFileError(edit->save(path));
  } else {
    return ActionResult::Succeed;
  }
}

ActionResult MainWindow::handleSaveFileError(const SaveResult &result) {
  if (auto newPath = std::get_if<QString>(&result)) {
    m_openFilePath.setValue(*newPath);
    return ActionResult::Succeed;
  } else if (auto err = std::get_if<std::optional<QString>>(&result)) {
    if (err->has_value()) {
      QMessageBox::critical(this, "文件保存失败", err->value());
    }
    return ActionResult::Fail;
  } else {
    // cancelled
    return ActionResult::Fail;
  }
}

ActionResult MainWindow::loadConfig(QWidget *parent) {
  auto result = gvb::init_machines();
  if (result.tag == gvb::InitMachineResult::Tag::Left) {
    QMessageBox::critical(
        parent, "错误",
        tr("配置文件加载失败：%1")
            .arg(QString::fromUtf8(result.left._0.data, result.left._0.len)));
    gvb::destroy_string(result.left._0);
    return ActionResult::Fail;
  } else {
    return ActionResult::Succeed;
  }
}

void MainWindow::setTitle() {
  if (auto edit = dynamic_cast<EditCapabilities *>(centralWidget())) {
    auto dirty = edit->m_dirty.value();
    if (m_openFilePath.value().isEmpty()) {
      setWindowTitle(tr(WINDOW_TITLE " - %1").arg(dirty ? "*" : ""));
    } else {
      auto name = QFileInfo(m_openFilePath.value()).fileName();
      setWindowTitle(
          tr(WINDOW_TITLE " - %1%2").arg(name).arg(dirty ? "*" : ""));
    }
  } else {
    if (m_openFilePath.value().isEmpty()) {
      setWindowTitle(tr(WINDOW_TITLE));
    } else {
      auto name = QFileInfo(m_openFilePath.value()).fileName();
      setWindowTitle(tr(WINDOW_TITLE " - %1").arg(name));
    }
  }
}