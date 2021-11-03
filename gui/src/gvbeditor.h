#pragma once

#include "capability.h"
#include "gvb.h"
#include "tool.h"
#include <QWidget>
#include <vector>
#include <variant>
#include <string>

class QAction;
class QToolBar;
class ScintillaEdit;
class QShowEvent;
class QTimerEvent;

namespace Scintilla {
  class NotificationData;
}

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
  gvb::Severity severity;
  std::string message;
};

struct GvbEditor : Tool, EditCapabilities, FileCapabilities {
private:
  Q_OBJECT

public:
  GvbEditor(QWidget *parent = nullptr);
  ~GvbEditor();

protected:
  void timerEvent(QTimerEvent *) override;

private:
  void initUi();
  void initEdit();
  QToolBar *initToolBar();
  void computeDiagnostics();

signals:
  void updateDiagnostics(std::vector<Diagnostic>);

public slots:
  ActionResult save(const QString &);
  void create();
  void find();
  void replace();
  void cut();
  void copy();
  void paste();
  void undo();
  void redo();
  ActionResult load(const QString &);
  bool canLoad(const QString &) const;

private slots:
  void notified(Scintilla::NotificationData *);
  void diagnosticsUpdated(std::vector<Diagnostic>);

private:
  QAction *m_actStart;
  QAction *m_actStop;
  ScintillaEdit *m_edit;
  gvb::Document *m_doc;
  bool m_textLoaded;
  int m_timerModify;
  std::vector<std::variant<InsertText, DeleteText>> m_edits;
  std::vector<Diagnostic> m_diagnostics;
};

Q_DECLARE_METATYPE(std::vector<Diagnostic>);