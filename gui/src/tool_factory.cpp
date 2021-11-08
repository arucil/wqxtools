#include "tool_factory.h"

static std::map<QString, std::function<ToolCtor>> toolFactories;

static std::map<QString, std::set<QString>> extensions;

std::optional<std::function<ToolCtor>>
ToolFactoryRegistry::get(const QString &ext) {
  auto it = toolFactories.find(ext.toLower());
  if (it == toolFactories.end()) {
    return {};
  } else {
    return it->second;
  }
}

void ToolFactoryRegistry::registerFactory(
    const QString &name, const ToolFactory &factory) {
  for (const auto &ext : factory.extensions) {
    toolFactories[ext] = factory.ctor;
  }

  for (const auto &ext : factory.extensions) {
    extensions[name].insert(ext.toLower());
  }
}

const std::map<QString, std::set<QString>> &
ToolFactoryRegistry::getExtensions() {
  return extensions;
}