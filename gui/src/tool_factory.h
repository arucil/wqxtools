#pragma once

#include <QString>
#include <functional>
#include <map>
#include <optional>
#include <set>

class Tool;
class QWidget;

typedef Tool *ToolCtor(QWidget *);

struct ToolFactory {
  std::set<QString> extensions;
  std::function<ToolCtor> ctor;
};

struct ToolFactoryRegistry {
  static std::optional<std::function<ToolCtor>> get(const QString &ext);

  static void registerFactory(const QString &name, const ToolFactory &);

  static const std::map<QString, std::set<QString>> &getExtensions();
};