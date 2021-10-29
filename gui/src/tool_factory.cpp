#include "tool_factory.h"

static std::map<QString, std::function<ToolCtor>> toolFactories;

static std::map<QString, std::vector<QString>> extensions;

std::optional<std::function<ToolCtor>>
ToolFactoryRegistry::get(const QString &ext) {
  auto it = toolFactories.find(ext.toLower());
  if (it == toolFactories.end()) {
    return std::make_optional<decltype(it->second)>();
  } else {
    return std::make_optional(it->second);
  }
}

void ToolFactoryRegistry::registerFactory(
    const QString &name, const ToolFactory &factory) {
  for (const auto &ext : factory.extensions) {
    toolFactories[ext] = factory.ctor;
  }

  extensions[name] = factory.extensions;
}

const std::map<QString, std::vector<QString>> &
ToolFactoryRegistry::getExtensions() {
  return extensions;
}