#pragma once

#include <QMainWindow>

#include "capability.h"
#include "gvbeditor.h"
#include "util.h"

class QWidget;
class QCloseEvent;
class QString;
class QAction;
class QString;
class Tool;

class MainWindow: public QMainWindow {
  Q_OBJECT

public:
  MainWindow(QWidget *parent = nullptr);

  static ActionResult loadConfig(QWidget *parent);

private:
  void initUi();
  void initMenu();
  ActionResult confirmSaveIfDirty(Tool *);

private slots:
  void openFile();
  void createFile();
  ActionResult saveFile();
  ActionResult saveFileAs(bool save = false);
  void setTitle();

private:
  void openFileByPath(const QString &);
  ActionResult handleSaveFileError(const SaveResult &);

protected:
  void closeEvent(QCloseEvent *) override;

private:
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
};