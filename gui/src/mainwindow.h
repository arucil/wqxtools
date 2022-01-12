#pragma once

#include <QMainWindow>
#include <QVector>

#include "capability.h"
#include "gvb/gvbeditor.h"
#include "util.h"

class QMenu;
class QWidget;
class QCloseEvent;
class QString;
class QAction;
class QString;
class ToolWidget;
class Tool;
class QScreen;
class QDragEnterEvent;
class QDropEvent;

class MainWindow: public QMainWindow {
  Q_OBJECT

public:
  MainWindow(QWidget *parent = nullptr);

  static ActionResult loadConfig(QWidget *parent);

private:
  void initUi();
  void initMenu();
  ActionResult confirmSaveIfDirty(ToolWidget *);

private slots:
  void openFile();
  void createFile(const Tool &);
  ActionResult saveFile();
  ActionResult saveFileAs(bool save = false);
  void setTitle();

private:
  void openFileByPath(const QString &, QScreen *);
  void openFileByPath(const QString &);
  ActionResult handleSaveFileError(const SaveResult &);
  void setupTool(ToolWidget *);
  void replaceTool(ToolWidget *);

protected:
  void closeEvent(QCloseEvent *) override;
  void dragEnterEvent(QDragEnterEvent *) override;
  void dropEvent(QDropEvent *) override;

private:
  QMenu *m_mnuEdit;

  QAction *m_actOpen;
  QAction *m_actSave;
  QAction *m_actSaveAs;

  QAction *m_actUndo;
  QAction *m_actRedo;
  QAction *m_actCopy;
  QAction *m_actCut;
  QAction *m_actPaste;
  QAction *m_actFind;
  QAction *m_actReplace;

  QAction *m_actStart;
  QAction *m_actStop;

  StrValue m_openFilePath;
  BoolValue m_loaded;

  QVector<QAction *> m_extraEditActions;
};