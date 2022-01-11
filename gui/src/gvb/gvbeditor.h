#pragma once

#include <api.h>

#include <QStateMachine>
#include <QVector>
#include <string>
#include <variant>

#include "../capability.h"
#include "../tool.h"

class QComboBox;
class EmojiSelector;
class QLabel;
class QWidget;
class QStatusBar;
class QTimerEvent;
class QKeyEvent;
class TextChange;
class CodeEditor;
class QToolBar;
class GvbSimWindow;
class SearchBar;

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
  void initStatusBar();
  void computeDiagnostics();
  void updateStartAction(QState *);
  void loadMachNames();
  void syncMachName(bool skipSelection);
  void syncMachNameSelection();

protected:
  void timerEvent(QTimerEvent *) override;
  void keyPressEvent(QKeyEvent *) override;

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
  void showMessage(const QString &, int ms);
  void showErrorMessage(const QString &, int ms);
  void setMachineName(int index);

private:
  CodeEditor *m_edit;
  api::GvbDocument *m_doc;
  bool m_textLoaded;
  bool m_needSyncMach;
  int m_timerModify;
  int m_timerError;
  QVector<std::variant<InsertText, DeleteText>> m_edits;
  QStateMachine m_stateMachine;
  GvbSimWindow *m_gvbsim;
  QString m_filePath;
  QToolBar *m_toolBar;
  QStatusBar *m_statusBar;
  SearchBar *m_searchBar;
  QLabel *m_errorLabel;
  QComboBox *m_machNames;
  EmojiSelector *m_emojiSelector;
};