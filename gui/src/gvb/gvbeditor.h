#pragma once

#include <api.h>

#include <QStateMachine>
#include <QVector>
#include <cstdint>
#include <string>
#include <variant>

#include "../capability.h"
#include "../tool.h"

using std::uint16_t;

class QTableView;
class ErrorListModel;
class QDialog;
class QToolButton;
class QComboBox;
class EmojiSelector;
class QStatusBar;
class QTimerEvent;
class QKeyEvent;
class TextChange;
class CodeEditor;
class QToolBar;
class GvbSimWindow;
class SearchBar;
class QPushButton;
class SyntaxStyle;

struct InsertText {
  size_t pos;
  std::string str;
};

struct DeleteText {
  size_t pos;
  size_t len;
};

class GvbEditor:
  public ToolWidget,
  public EditCapabilities,
  public FileCapabilities,
  public ProgramCapabilities {
  Q_OBJECT

public:
  GvbEditor(QWidget *parent = nullptr);
  ~GvbEditor();

  QList<QAction *> extraActions() const;
  void setContextMenuActions(const QList<QAction *> &) override;

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
  void addLabel(api::GvbLabelTarget);

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
  void addLabelCurLine();
  void addLabelPrevLine();
  void addLabelNextLine();
  LoadResult load(const QString &) override;
  bool canLoad(const QString &) const override;
  const char *type() const override;
  QSize preferredWindowSize() const override;
  void tryStartPause(QWidget *sender);

private slots:
  void modified();
  void textChanged(const TextChange &);
  void setMachineName(int index);
  void contextMenu(const QPoint &localPos);
  void showEmojiSelector();
  void applyEdits();
  void syncMachNameEdit();
  void showRelabelDialog();
  void relabel(uint16_t start, uint16_t inc);
  void toggleErrorList();

private:
  CodeEditor *m_edit;
  api::GvbDocument *m_doc;
  bool m_textLoaded;
  int m_timerModify;
  QVector<std::variant<InsertText, DeleteText>> m_edits;
  QStateMachine m_stateMachine;
  GvbSimWindow *m_gvbsim;
  QString m_filePath;
  QToolBar *m_toolBar;
  QStatusBar *m_statusBar;
  QDialog *m_relabelDlg;
  SearchBar *m_searchBar;
  QComboBox *m_machNames;
  QToolButton *m_btnEmoji;
  EmojiSelector *m_emojiSelector;
  QAction *m_actAddLabelCurLine;
  QAction *m_actAddLabelPrevLine;
  QAction *m_actAddLabelNextLine;
  QAction *m_actRelabel;
  QList<QAction *> m_ctxMenuActions;
  QPushButton *m_errBtn;
  QTableView *m_errorView;
  ErrorListModel *m_errorListModel;
};