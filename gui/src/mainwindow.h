#pragma once

#include <QMainWindow>

class QAction;

class MainWindow : public QMainWindow {
  Q_OBJECT

public:
  MainWindow(QWidget *parent = nullptr);

private:
  void initUi();
  void initMenu();

private slots:
  void openFile();
  void createFile();
  void saveFile();
  void saveFileAs();

protected:
  void closeEvent(QCloseEvent *) override;

private:
  QAction *m_actSave;
  QAction *m_actSaveAs;
  QAction *m_actExit;
  QAction *m_actUndo;
  QAction *m_actRedo;
  QAction *m_actCopy;
  QAction *m_actCut;
  QAction *m_actPaste;
  QAction *m_actFind;
  QAction *m_actReplace;
};