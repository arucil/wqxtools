#pragma once

#include <QWidget>

class QAction;

class GvbEditor : public QWidget {
public:
  GvbEditor(QWidget *parent = nullptr);
  ~GvbEditor();

private:
  QAction *m_actSave;
  QAction *m_actStart;
  QAction *m_actStop;
};