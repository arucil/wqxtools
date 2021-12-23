#pragma once

#include <QStateMachine>
#include <QStatusBar>
#include <QWidget>
#include <algorithm>
#include <string>
#include <variant>
#include <vector>

#include "../capability.h"
#include "../tool.h"
#include "api.h"

class QCodeEditor;
class QAction;
class QToolBar;
class QShowEvent;
class QTimerEvent;
class QLabel;
class GvbSimWindow;
class QCloseEvent;

struct InsertText {
  size_t pos;
  std::string str;
};

struct DeleteText {
  size_t pos;
  size_t len;
};

struct Diagnostic {
  size_t line;
  size_t start;
  size_t end;
  api::GvbSeverity severity;
  std::string message;
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
  void updateDiagnostics(std::vector<Diagnostic>);
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
  void notified(void *);
  void diagnosticsUpdated(std::vector<Diagnostic>);
  void modified();

private:
  QCodeEditor *m_edit;
  api::GvbDocument *m_doc;
  bool m_textLoaded;
  bool m_timerModify;
  std::vector<std::variant<InsertText, DeleteText>> m_edits;
  std::vector<Diagnostic> m_diagnostics;
  QStateMachine m_stateMachine;
  GvbSimWindow *m_gvbsim;
  QString m_filePath;
};

Q_DECLARE_METATYPE(std::vector<Diagnostic>);