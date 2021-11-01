#pragma once

#include <QWidget>
#include "util.h"

struct Tool : public QWidget {
  Tool(QWidget *parent = nullptr) : QWidget(parent) {}
  virtual ActionResult load(const QString &) = 0;
  virtual bool canLoad(const QString &) const = 0;
};