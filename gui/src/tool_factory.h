#pragma once

#include <functional>
#include <optional>
#include <set>

class Tool;
class QWidget;
class QString;

typedef Tool *ToolCtor(QWidget *);

struct ToolFactory {
  std::set<QString> extensions;
  std::function<ToolCtor> ctor;
};

struct ToolFactoryRegistry {
  static std::optional<std::function<ToolCtor>> get(const QString &ext);

  static void registerFactory(const QString &name, const ToolFactory &);

  static const QString &openFileFilter();

  static QString saveFileFilter(const QString &ext);
};