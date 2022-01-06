#pragma once

#include <QSet>
#include <QMap>
#include <QString>

class ToolWidget;
class QWidget;

using ToolCtor = ToolWidget *(QWidget *);
using ToolTest = bool (ToolWidget *);

struct ToolConfig {
  QSet<QString> extensions;
  ToolCtor *ctor;
  ToolTest *test;
  bool canCreate;
};

struct Tool {
  ToolCtor *ctor;
  ToolTest *test;
};

struct ToolRegistry {
  static ToolCtor *getCtorByExt(const QString &ext);

  static void registerTool(const QString &name, const ToolConfig &);

  static const QMap<QString, Tool> &createFileTools();

  static const QString &openFileFilter();

  static QString saveFileFilter(const QString &ext);
};