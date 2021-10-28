#pragma once

#include <QWidget>
#include "capability.h"
#include "gvb.h"

class QAction;
class QToolBar;
class ScintillaEdit;
class QShowEvent;

class GvbEditor : public QWidget,
                  public EditCapabilities,
                  public FileCapabilities {

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
  void find();
  void replace();
  void cut();
  void copy();
  void paste();
  void undo();
  void redo();

private:
  QAction *m_actStart;
  QAction *m_actStop;
  ScintillaEdit *m_edit;
  gvb::Document *m_doc;
};