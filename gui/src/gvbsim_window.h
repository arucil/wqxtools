#pragma once

#include <QMainWindow>

class GvbSimWindow : public QMainWindow {
  Q_OBJECT

public:
  GvbSimWindow(QWidget *parent = nullptr);
  ~GvbSimWindow();
};