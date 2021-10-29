#pragma once

#include "capability.h"
#include "gvb.h"
#include "tool.h"
#include <QWidget>

class QAction;
class QToolBar;
class ScintillaEdit;
class QShowEvent;

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
  void save();
  void saveAs(const QString &);
  void create();
  void find();
  void replace();
  void cut();
  void copy();
  void paste();
  void undo();
  void redo();
  void load(const QString &);

private:
  QAction *m_actStart;
  QAction *m_actStop;
  ScintillaEdit *m_edit;
  gvb::Document *m_doc;
  bool m_dirty;
};