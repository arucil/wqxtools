#pragma once

#include "capability.h"
#include "gvb.h"
#include "tool.h"
#include <QWidget>

class QAction;
class QToolBar;
class ScintillaEdit;
class QShowEvent;

namespace Scintilla {
  class NotificationData;
}

struct GvbEditor : Tool, EditCapabilities, FileCapabilities {
private:
  Q_OBJECT

public:
  GvbEditor(QWidget *parent = nullptr);
  ~GvbEditor();

private:
  void initUi();
  void initEdit();
  QToolBar *initToolBar();

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

private:
  QAction *m_actStart;
  QAction *m_actStop;
  ScintillaEdit *m_edit;
  gvb::Document *m_doc;
  bool m_textLoaded;
};