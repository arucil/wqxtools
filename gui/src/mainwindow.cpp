#include "mainwindow.h"

#include <QApplication>
#include <QCloseEvent>
#include <QDragEnterEvent>
#include <QDropEvent>
#include <QFileDialog>
#include <QFileInfo>
#include <QFrame>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QLabel>
#include <QMenu>
#include <QMenuBar>
#include <QMessageBox>
#include <QMimeData>
#include <QNetworkAccessManager>
#include <QNetworkReply>
#include <QNetworkRequest>
#include <QPushButton>
#include <QScreen>
#include <QTimer>

#include "about_dialog.h"
#include "action.h"
#include "api.h"
#include "config.h"
#include "toast.h"
#include "tool.h"
#include "tool_registry.h"
#include "value.h"

#define WINDOW_TITLE "WQX 工具箱"
#define UNNAMED "未命名"
#define STYLE_DIR "styles"

#define VERSION_API_ENDPOINT \
  "https://gitlab.com/api/v4/projects/32814745/releases"

MainWindow::MainWindow(QWidget *parent) :
  QMainWindow(parent),
  m_networkMan(new QNetworkAccessManager(this)) {
  m_networkMan->setTransferTimeout(3000);

  initUi();

  resize(400, 340);

  QTimer::singleShot(0, this, [this] {
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

    centerWindow(this, qApp->primaryScreen());

    auto args = QCoreApplication::arguments();
    if (args.length() > 2) {
      QMessageBox::critical(this, "运行参数错误", "运行参数过多");
    } else if (args.length() == 2) {
      openFileByPath(args.at(1), qApp->primaryScreen());
    }

    checkNewVersion(false);
  });
}

void MainWindow::initUi() {
  setAcceptDrops(true);

  initMenu();

  auto help = new QLabel(
    "<p>点击菜单 [文件] -> [打开] 打开文件<br>"
    "或拖动文件到此窗口</p>");
  help->setFrameStyle(QFrame::Box);
  help->setAlignment(Qt::AlignCenter | Qt::AlignVCenter);
  help->setContentsMargins(20, 20, 20, 20);
  setCentralWidget(help);

  connect(&m_openFilePath, &StrValue::changed, this, &MainWindow::setTitle);
  setTitle();

  // NOTE pass reference for QueuedConnection is safe, because the passed value
  // will be copied, the original reference will never be used.
  // See https://forum.qt.io/topic/53780/solved-safety-of-references-as-arguments-to-emitted-signals/2
  connect(
    MessageBus::instance(),
    &MessageBus::newMessage,
    this,
    &MainWindow::showMessage,
    Qt::QueuedConnection);

  m_toast = new Toast(this);
}

void MainWindow::initMenu() {
  auto mnuFile = menuBar()->addMenu("文件(&F)");

  m_actOpen = mnuFile->addAction("打开(&O)");
  m_actOpen->setShortcut(Qt::CTRL | Qt::Key_O);
  connect(m_actOpen, &QAction::triggered, this, &MainWindow::openFile);

  auto mnuNew = mnuFile->addMenu("新建(&N)");
  const auto &tools = ToolRegistry::createFileTools();
  for (auto it = tools.constKeyValueBegin(); it != tools.constKeyValueEnd();
       it++) {
    auto actNew = mnuNew->addAction(it->first);
    auto tool = it->second;
    connect(actNew, &QAction::triggered, this, [tool, this] {
      createFile(tool);
    });
  }

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

  m_mnuEdit = menuBar()->addMenu("编辑(&E)");

  m_actUndo = m_mnuEdit->addAction("撤销");
  m_actUndo->setShortcut(Qt::CTRL | Qt::Key_Z);

  m_actRedo = m_mnuEdit->addAction("重做");
  m_actRedo->setShortcut(Qt::CTRL | Qt::Key_Y);

  m_mnuEdit->addSeparator();

  m_actCopy = m_mnuEdit->addAction("复制");
  m_actCopy->setShortcut(Qt::CTRL | Qt::Key_C);

  m_actCut = m_mnuEdit->addAction("剪切");
  m_actCut->setShortcut(Qt::CTRL | Qt::Key_X);

  m_actPaste = m_mnuEdit->addAction("粘贴");
  m_actPaste->setShortcut(Qt::CTRL | Qt::Key_V);

  m_mnuEdit->addSeparator();

  m_actSelectAll = m_mnuEdit->addAction("全选");
  m_actSelectAll->setShortcut(Qt::CTRL | Qt::Key_A);

  m_mnuEdit->addSeparator();

  m_actFind = m_mnuEdit->addAction("查找");
  m_actFind->setShortcut(Qt::CTRL | Qt::Key_F);

  m_actReplace = m_mnuEdit->addAction("替换");
  m_actReplace->setShortcut(Qt::CTRL | Qt::Key_R);

  auto mnuProg = menuBar()->addMenu("程序(&P)");

  m_actStart = mnuProg->addAction("运行");
  m_actStart->setShortcut(Qt::Key_F5);

  m_actStop = mnuProg->addAction("停止");
  m_actStop->setShortcut(Qt::CTRL | Qt::Key_F7);

  mnuProg->addSeparator();

  auto actConfig = mnuProg->addAction("重新加载配置文件");
  connect(actConfig, &QAction::triggered, this, [this] { loadConfig(this); });

  auto mnuHelp = menuBar()->addMenu("帮助");

  auto actCheckVer = mnuHelp->addAction("检查新版本");
  connect(actCheckVer, &QAction::triggered, this, [this] {
    showMessage("正在检查版本更新", 1000, MessageType::Info);
    checkNewVersion(true);
  });

  mnuHelp->addSeparator();

  auto actAbout = mnuHelp->addAction("关于");
  connect(actAbout, &QAction::triggered, this, [this] {
    AboutDialog(this).exec();
  });

  auto actAboutQt = mnuHelp->addAction("关于 Qt");
  connect(actAboutQt, &QAction::triggered, this, [this] {
    QMessageBox::aboutQt(this, "关于 Qt");
  });
}

void MainWindow::closeEvent(QCloseEvent *event) {
  auto widget = qobject_cast<ToolWidget *>(centralWidget());
  if (widget && confirmSaveIfDirty(widget) == ActionResult::Fail) {
    event->ignore();
  }
}

void MainWindow::openFile() {
  auto path = QFileDialog::getOpenFileName(
    this,
    "",
    "",
    ToolRegistry::openFileFilter(),
    nullptr,
    QFileDialog::Option::DontResolveSymlinks
      | QFileDialog::Option::DontUseNativeDialog);
  openFileByPath(path, screen());
}

void MainWindow::openFileByPath(const QString &path) {
  openFileByPath(path, screen());
}

void MainWindow::openFileByPath(const QString &path, QScreen *screen) {
  if (path.isEmpty()) {
    return;
  }

  auto widget = qobject_cast<ToolWidget *>(centralWidget());
  if (confirmSaveIfDirty(widget) == ActionResult::Fail) {
    return;
  }

  auto fileinfo = QFileInfo(path);
  if (!fileinfo.exists()) {
    QMessageBox::critical(
      this,
      "文件打开失败",
      QString("文件不存在：%1").arg(path));
    return;
  }

  auto ext = fileinfo.suffix();
  if (ext.isEmpty()) {
    QMessageBox::critical(
      this,
      "文件打开失败",
      "文件缺少后缀名，无法识别文件类型");
    return;
  }

  auto isNew = false;
  if (!widget || !widget->canLoad(path)) {
    auto ctor = ToolRegistry::getCtorByExt(ext.toLower());
    if (!ctor) {
      QMessageBox::critical(
        this,
        "文件打开失败",
        QString("不支持的文件类型：") + ext.toLower());
      return;
    }

    isNew = true;
    widget = ctor(this);
    replaceTool(widget);

    resize(widget->preferredWindowSize());
    centerWindow(this, screen);
  }

  QTimer::singleShot(0, widget, [widget, path, this] {
    auto result = widget->load(path);
    if (auto err = std::get_if<QString>(&result)) {
      QMessageBox::critical(this, "文件打开失败", *err);
      replaceTool(nullptr);
      m_openFilePath.setValue(QString());
      m_loaded.setValue(false);
    } else {
      m_loaded.setValue(true);
    }
  });

  m_openFilePath.setValue(fileinfo.absoluteFilePath());

  if (isNew) {
    setupTool(widget);
  }
}

void MainWindow::setupTool(ToolWidget *widget) {
  connect(
    widget,
    &ToolWidget::fileDropped,
    this,
    qOverload<const QString &>(&MainWindow::openFileByPath));

  auto fileCap = dynamic_cast<FileCapabilities *>(widget);
  m_actSave->setEnabled(fileCap != nullptr);
  m_actSaveAs->setEnabled(fileCap != nullptr);

  if (auto editor = dynamic_cast<EditCapabilities *>(widget)) {
    m_actCopy->setEnabled(editor->m_actCopy);
    connect(
      editor->m_actCopy,
      &Action::enabledChanged,
      m_actCopy,
      &QAction::setEnabled);
    m_actCut->setEnabled(editor->m_actCut);
    connect(
      editor->m_actCut,
      &Action::enabledChanged,
      m_actCut,
      &QAction::setEnabled);
    m_actPaste->setEnabled(editor->m_actPaste);
    connect(
      editor->m_actPaste,
      &Action::enabledChanged,
      m_actPaste,
      &QAction::setEnabled);
    m_actSelectAll->setEnabled(editor->m_actSelectAll);
    connect(
      editor->m_actSelectAll,
      &Action::enabledChanged,
      m_actSelectAll,
      &QAction::setEnabled);
    m_actUndo->setEnabled(editor->m_actUndo);
    connect(
      editor->m_actUndo,
      &Action::enabledChanged,
      m_actUndo,
      &QAction::setEnabled);
    m_actRedo->setEnabled(editor->m_actRedo);
    connect(
      editor->m_actRedo,
      &Action::enabledChanged,
      m_actRedo,
      &QAction::setEnabled);
    m_actFind->setEnabled(true);
    m_actReplace->setEnabled(true);

    connect(
      m_actCopy,
      &QAction::triggered,
      editor->m_actCopy,
      &QAction::trigger);
    connect(m_actCut, &QAction::triggered, editor->m_actCut, &QAction::trigger);
    connect(
      m_actPaste,
      &QAction::triggered,
      editor->m_actPaste,
      &QAction::trigger);
    connect(
      m_actUndo,
      &QAction::triggered,
      editor->m_actUndo,
      &QAction::trigger);
    connect(
      m_actRedo,
      &QAction::triggered,
      editor->m_actRedo,
      &QAction::trigger);
    connect(
      m_actSelectAll,
      &QAction::triggered,
      editor->m_actSelectAll,
      &QAction::trigger);
    connect(
      m_actFind,
      &QAction::triggered,
      editor->m_actFind,
      &QAction::trigger);
    connect(
      m_actReplace,
      &QAction::triggered,
      editor->m_actReplace,
      &QAction::trigger);

    connect(&editor->m_dirty, &BoolValue::changed, this, &MainWindow::setTitle);

    auto extraActions = editor->extraActions();
    if (!extraActions.isEmpty()) {
      m_extraEditActions.push_back(m_mnuEdit->addSeparator());
      m_mnuEdit->addActions(extraActions);
    }

    editor->setContextMenuActions(m_mnuEdit->actions());
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
    progCap->m_stStarted->assignProperty(m_actStart, "text", "暂停");
    progCap->m_stStopped->assignProperty(m_actStart, "text", "运行");
    progCap->m_stPaused->assignProperty(m_actStart, "text", "继续");
    progCap->m_stStarted->assignProperty(m_actStop, "enabled", true);
    progCap->m_stPaused->assignProperty(m_actStop, "enabled", true);
    progCap->m_stStopped->assignProperty(m_actStop, "enabled", false);
    progCap->m_stStopped->assignProperty(m_actOpen, "enabled", true);
    progCap->m_stStarted->assignProperty(m_actOpen, "enabled", false);
    progCap->m_stPaused->assignProperty(m_actOpen, "enabled", false);
    m_actStart->setEnabled(true);
    connect(
      m_actStart,
      &QAction::triggered,
      progCap->m_actStart,
      &QAction::trigger);
    connect(
      m_actStop,
      &QAction::triggered,
      progCap->m_actStop,
      &QAction::trigger);
  } else {
    m_actStart->setEnabled(false);
    m_actStop->setEnabled(false);
    m_actStart->setText("运行");
  }

  if (fileCap && fileCap->m_actSave != nullptr) {
    connect(
      fileCap->m_actSave,
      &QAction::triggered,
      this,
      &MainWindow::saveFile);
  }
}

void MainWindow::replaceTool(ToolWidget *tool) {
  for (auto act : m_extraEditActions) {
    m_mnuEdit->removeAction(act);
  }
  m_extraEditActions.clear();
  setCentralWidget(tool);
}

ActionResult MainWindow::confirmSaveIfDirty(ToolWidget *widget) {
  if (auto oldWidget = dynamic_cast<EditCapabilities *>(widget)) {
    if (oldWidget->m_dirty.value()) {
      auto btn = QMessageBox::question(
        this,
        "文件改动",
        QString("文件 %1 有改动，是否保存？")
          .arg(QFileInfo(m_openFilePath.value()).fileName()),
        QMessageBox::StandardButton::Yes | QMessageBox::StandardButton::No
          | QMessageBox::StandardButton::Cancel);
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

void MainWindow::createFile(const Tool &tool) {
  auto widget = qobject_cast<ToolWidget *>(centralWidget());
  if (confirmSaveIfDirty(widget) == ActionResult::Fail) {
    return;
  }

  auto isNew = false;
  if (!(tool.test)(widget)) {
    isNew = true;
    widget = (tool.ctor)(this);
    replaceTool(widget);

    resize(widget->preferredWindowSize());
    centerWindow(this, screen());
  }

  QTimer::singleShot(0, widget, [widget, this] {
    dynamic_cast<FileCapabilities *>(widget)->create();
  });

  m_openFilePath.setValue(QString());

  if (isNew) {
    setupTool(widget);
  }
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
    const auto &lastPath = m_openFilePath.value();
    auto ext = QFileInfo(lastPath).suffix().toLower();
    if (ext.isEmpty()) {
      ext = edit->defaultExt();
    }
    auto path = QFileDialog::getSaveFileName(
      this,
      save ? "保存文件" : "另存为",
      lastPath.isEmpty() ? QString(UNNAMED) + "." + ext : lastPath,
      ToolRegistry::saveFileFilter(ext),
      nullptr,
      QFileDialog::Option::DontResolveSymlinks
        | QFileDialog::Option::DontUseNativeDialog);
    if (path.isEmpty()) {
      return ActionResult::Fail;
    }

    if (QFileInfo(path).suffix().isEmpty()) {
      path += '.';
      if (lastPath.isEmpty()) {
        path += edit->defaultExt();
      } else {
        path = QFileInfo(lastPath).suffix();
      }
    }

    m_openFilePath.setValue(path);
    return handleSaveFileError(edit->save(path));
  } else {
    return ActionResult::Succeed;
  }
}

ActionResult MainWindow::handleSaveFileError(const SaveResult &result) {
  if (auto newPath = std::get_if<0>(&result)) {
    m_openFilePath.setValue(*newPath);
    return ActionResult::Succeed;
  } else if (auto err = std::get_if<1>(&result)) {
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
  {
    auto result = api::gvb_init_machines();
    if (result.tag == api::GvbInitMachineResult::Tag::Left) {
      QMessageBox::critical(
        parent,
        "错误",
        QString("机型配置文件加载失败：%1")
          .arg(QString::fromUtf8(result.left._0.data, result.left._0.len)));
      api::destroy_string(result.left._0);
      return ActionResult::Fail;
    }
  }

  {
    auto result = api::load_config();
    if (result.tag == api::LoadConfigResult::Tag::Left) {
      QMessageBox::critical(
        parent,
        "错误",
        QString("配置文件加载失败：%1")
          .arg(QString::fromUtf8(result.left._0.data, result.left._0.len)));
      api::destroy_string(result.left._0);
      return ActionResult::Fail;
    }

    if (
      api::config()->gvb.editor.style.tag
      == api::Maybe<api::Utf8String>::Tag::Just) {
      auto s = api::config()->gvb.editor.style.just._0;
      auto style = QString::fromUtf8(s.data, s.len);
      auto styleDir = getSystemDir(STYLE_DIR);
      QFile styleFile(styleDir + QDir::separator() + style + ".xml");
      if (!styleFile.open(QIODevice::ReadOnly | QIODevice::Text)) {
        QMessageBox::critical(
          parent,
          "错误",
          QString("加载 style XML 文件失败：%1\n错误信息：%2")
            .arg(styleFile.fileName(), styleFile.errorString()));
        return ActionResult::Fail;
      }

      auto result = SyntaxStyle::load(styleFile);
      if (auto err = std::get_if<0>(&result)) {
        QMessageBox::critical(
          parent,
          "错误",
          QString("加载 style XML 文件失败：%1\n错误信息：%2")
            .arg(styleFile.fileName(), *err));
        return ActionResult::Fail;
      }

      Config::instance()->setStyle(std::get<1>(result));
    } else {
      Config::instance()->setStyle({});
    }
  }

  emit Config::instance()->configChanged();

  return ActionResult::Succeed;
}

void MainWindow::setTitle() {
  auto &path = m_openFilePath.value();
  if (auto edit = dynamic_cast<EditCapabilities *>(centralWidget())) {
    auto dirty = edit->m_dirty.value();
    if (path.isEmpty()) {
      setWindowTitle(
        QString(WINDOW_TITLE " - " UNNAMED "%1").arg(dirty ? "*" : ""));
    } else {
      auto name = QFileInfo(path).fileName();
      setWindowTitle(
        QString(WINDOW_TITLE " - %1%2").arg(name, dirty ? "*" : ""));
    }
  } else {
    if (path.isEmpty()) {
      setWindowTitle(QString(WINDOW_TITLE));
    } else {
      auto name = QFileInfo(path).fileName();
      setWindowTitle(QString(WINDOW_TITLE " - %1").arg(name));
    }
  }
}

void MainWindow::dragEnterEvent(QDragEnterEvent *ev) {
  if (ev->mimeData()->hasUrls()) {
    ev->acceptProposedAction();
  }
}

void MainWindow::dropEvent(QDropEvent *ev) {
  const auto urls = ev->mimeData()->urls();
  for (auto &url : urls) {
    if (url.isLocalFile()) {
      openFileByPath(url.toLocalFile(), screen());
    }
  }
}

void MainWindow::checkNewVersion(bool manual) {
  auto reply = m_networkMan->get(QNetworkRequest(QUrl(VERSION_API_ENDPOINT)));
  connect(reply, &QNetworkReply::finished, [=] {
    reply->deleteLater();

    if (reply->error()) {
      if (manual) {
        QString msg;
        switch (reply->error()) {
          case QNetworkReply::TimeoutError:
            msg = "连接超时";
            break;
          case QNetworkReply::TemporaryNetworkFailureError:
            msg = "网络断开";
            break;
          default:
            msg = reply->errorString();
            break;
        }
        QMessageBox::critical(this, "错误", QString("检查版本失败：") + msg);
      }

      return;
    }
    auto resp = reply->readAll();
    QJsonParseError error;
    auto json = QJsonDocument::fromJson(resp, &error);
    if (error.error != QJsonParseError::NoError) {
      if (manual) {
        QMessageBox::critical(this, "错误", "检查版本失败：JSON parse error");
      }
      return;
    }

    auto tag = json.array().at(0).toObject()["tag_name"].toString().toUtf8();
    auto result =
      api::is_new_version({tag.data(), static_cast<size_t>(tag.size())});
    if (result.tag == api::Maybe<bool>::Tag::Nothing) {
      if (manual) {
        QMessageBox::critical(
          this,
          "错误",
          "检查版本失败：release tag_name is not semver");
      }
      return;
    }

    if (manual) {
      if (result.just._0) {
        notifyNewVersion(tag);
      } else {
        showMessage("已经是最新版本", 700, MessageType::Info);
      }
    } else if (result.just._0) {
      showMessage(
        "有新版本，请点击菜单 [帮助] -> [检查新版本] 查看新版本",
        1500,
        MessageType::Info);
    }
  });
}

void MainWindow::notifyNewVersion(const QString &tag) {
  auto reply = m_networkMan->get(
    QNetworkRequest(QUrl(QString(VERSION_API_ENDPOINT "/%1?include_html_description=true").arg(tag))));
  connect(reply, &QNetworkReply::finished, [=] {
    reply->deleteLater();
    if (reply->error()) {
      QString msg;
      switch (reply->error()) {
        case QNetworkReply::TimeoutError:
          msg = "连接超时";
          break;
        case QNetworkReply::TemporaryNetworkFailureError:
          msg = "网络断开";
          break;
        default:
          msg = reply->errorString();
          break;
      }
      QMessageBox::critical(
        this,
        "错误",
        QString("获取新版本信息失败：") + msg);
    }
    auto resp = reply->readAll();
    QJsonParseError error;
    auto json = QJsonDocument::fromJson(resp, &error);
    if (error.error != QJsonParseError::NoError) {
      QMessageBox::critical(
        this,
        "错误",
        "获取新版本信息失败：JSON parse error");
      return;
    }

    auto release = json.object();
    auto description = release["description_html"].toString();
    auto url = release["_links"].toObject()["self"].toString();

    m_toast->hide();

    QMessageBox::information(
      this,
      "新版本",
      QString("<h3>%1</h3><p>%2</p><a href=\"%3\">点击链接下载新版本</a>")
        .arg(tag, description, url));
  });
}

void MainWindow::showMessage(const QString &text, int ms, MessageType type) {
  switch (type) {
    case MessageType::Info:
      m_toast->showText(text, ms);
      break;
    case MessageType::Error:
      m_toast->showText(
        QString("<font color=\"red\">%1</font>").arg(text.toHtmlEscaped()),
        ms);
      break;
  }
}