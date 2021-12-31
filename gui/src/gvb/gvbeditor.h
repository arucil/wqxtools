#pragma once

#include <QStateMachine>
#include <QStatusBar>
#include <QWidget>
#include <string>
#include <variant>
#include <QVector>

#include "../capability.h"
#include "../tool.h"

class CodeEditor;
class QAction;
class QToolBar;
class QShowEvent;
class QTimerEvent;
class QLabel;
class GvbSimWindow;
class QCloseEvent;

namespace api {
  class GvbDocument;
}

struct InsertText {
  size_t pos;
  std::string str;
};

struct DeleteText {
  size_t pos;
  size_t len;
};

struct GvbEditor:
  Tool,
  EditCapabilities,
  FileCapabilities,
  ProgramCapabilities {
private:
  Q_OBJECT

public:
  GvbEditor(QWidget *parent = nullptr);
  ~GvbEditor();

private:
  void initUi();
  void initEdit();
  void initStateMachine();
  QToolBar *initToolBar();
  QStatusBar *initStatusBar();
  void computeDiagnostics();
  void updateStartAction(QState *);

signals:
  void start();
  void pause();
  void cont();
  void stop();

public slots:
  SaveResult save(const QString &) override;
  void create();
  void find();
  void replace();
  void cut();
  void copy();
  void paste();
  void undo();
  void redo();
  LoadResult load(const QString &) override;
  bool canLoad(const QString &) const override;
  QSize preferredWindowSize() const override;
  void tryStartPause(QWidget *sender);

private slots:
  void modified();

private:
  CodeEditor *m_edit;
  api::GvbDocument *m_doc;
  bool m_textLoaded;
  bool m_timerModify;
  QVector<std::variant<InsertText, DeleteText>> m_edits;
  QStateMachine m_stateMachine;
  GvbSimWindow *m_gvbsim;
  QString m_filePath;
};