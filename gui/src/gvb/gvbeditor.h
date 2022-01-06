#pragma once

#include <api.h>

#include <QStateMachine>
#include <QStatusBar>
#include <QVector>
#include <QWidget>
#include <string>
#include <variant>

#include "../capability.h"
#include "../tool.h"

class QTimerEvent;
class TextChange;
class CodeEditor;
class QAction;
class QToolBar;
class GvbSimWindow;

struct InsertText {
  size_t pos;
  std::string str;
};

struct DeleteText {
  size_t pos;
  size_t len;
};

struct GvbEditor:
  ToolWidget,
  EditCapabilities,
  FileCapabilities,
  ProgramCapabilities {
private:
  Q_OBJECT

public:
  GvbEditor(QWidget *parent = nullptr);
  ~GvbEditor();

  void showRuntimeError(const api::GvbExecResult::Error_Body &);

private:
  void initUi();
  void initEdit();
  void initStateMachine();
  void initToolBar();
  QStatusBar *initStatusBar();
  void computeDiagnostics();
  void updateStartAction(QState *);
  void showErrorToolTip(const QPoint &, const QString &);

protected:
  void timerEvent(QTimerEvent *) override;

signals:
  void start();
  void pause();
  void cont();
  void stop();

public slots:
  SaveResult save(const QString &) override;
  void create() override;
  const char *defaultExt() const;
  void find();
  void replace();
  void cut();
  void copy();
  void paste();
  void undo();
  void redo();
  LoadResult load(const QString &) override;
  bool canLoad(const QString &) const override;
  const char *type() const override;
  QSize preferredWindowSize() const override;
  void tryStartPause(QWidget *sender);

private slots:
  void modified();
  void textChanged(const TextChange &);

private:
  CodeEditor *m_edit;
  api::GvbDocument *m_doc;
  bool m_textLoaded;
  int m_timerModify;
  int m_timerError;
  QVector<std::variant<InsertText, DeleteText>> m_edits;
  QStateMachine m_stateMachine;
  GvbSimWindow *m_gvbsim;
  QString m_filePath;
  QToolBar *m_toolbar;
};