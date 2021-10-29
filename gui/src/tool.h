#pragma once

#include <QWidget>

struct Tool : public QWidget {
  Tool(QWidget *parent = nullptr) : QWidget(parent) {}
  virtual void load(const QString &) = 0;
};