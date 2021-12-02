#pragma once

#include <QDialog>

class AboutDialog: public QDialog {
  Q_OBJECT
public:
  AboutDialog(QWidget *parent = nullptr);
};