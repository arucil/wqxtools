#pragma once

#include <QSize>
#include <QWidget>

#include "util.h"

// succeeded
// failed: error message
using LoadResult = std::variant<Unit, QString>;

struct ToolWidget: public QWidget {
  Q_OBJECT

public:
  ToolWidget(QWidget *parent = nullptr) : QWidget(parent) {}

  virtual LoadResult load(const QString &) = 0;
  virtual bool canLoad(const QString &) const = 0;
  virtual const char *type() const = 0;

  virtual QSize preferredWindowSize() const = 0;

signals:
  void fileDropped(const QString &path);
};