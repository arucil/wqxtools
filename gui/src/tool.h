#pragma once

#include "util.h"
#include <QWidget>

// succeeded
// failed: error message
using LoadResult = std::variant<Unit, QString>;

struct Tool : public QWidget {
  Tool(QWidget *parent = nullptr) : QWidget(parent) {}
  virtual LoadResult load(const QString &) = 0;
  virtual bool canLoad(const QString &) const = 0;
};