#pragma once

#include <QMainWindow>

class QAction;

class MainWindow : public QMainWindow {
  Q_OBJECT

public:
  MainWindow(QWidget *parent = nullptr);

private:
  void setupUi();
  void setupMenu();

private slots:
  void openFile();
  void createFile();
  void saveFile();
  void saveFileAs();

protected:
  void closeEvent(QCloseEvent *) override;

private:
  QAction *m_mnuSave;
  QAction *m_mnuSaveAs;
  QAction *m_mnuExit;
};