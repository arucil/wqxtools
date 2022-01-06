#include "tool_registry.h"

#include <QString>
#include <optional>

using std::optional;

static QMap<QString, ToolCtor *> toolFactories;

static QMap<QString, QSet<QString>> extensions;

static QMap<QString, Tool> createFactories;

static optional<QString> openFileFilter;

static QMap<QString, QString> saveFileFilters;

ToolCtor *ToolRegistry::getCtorByExt(const QString &ext) {
  auto it = toolFactories.find(ext.toLower());
  if (it == toolFactories.end()) {
    return {};
  } else {
    return *it;
  }
}

void ToolRegistry::registerTool(
  const QString &name,
  const ToolConfig &config) {
  for (const auto &ext : config.extensions) {
    toolFactories.insert(ext, *config.ctor);
  }

  for (const auto &ext : config.extensions) {
    extensions[name].insert(ext.toLower());
  }

  if (config.canCreate) {
    createFactories.insert(name, {*config.ctor,*config.test});
  }
}

const QMap<QString, Tool> &ToolRegistry::createFileTools() {
  return createFactories;
}

const QString &ToolRegistry::openFileFilter() {
  if (::openFileFilter.has_value()) {
    return ::openFileFilter.value();
  }

  QString filter;
  auto semi = false;
  for (const auto &i : extensions.keys()) {
    if (semi) {
      filter += ";;";
    }
    semi = true;
    filter += i;
    filter += " (";
    for (auto &ext : extensions[i]) {
      filter += "*.";
      filter += ext;
      filter += " ";
    }
    filter += ")";
  }

  ::openFileFilter = filter;

  return ::openFileFilter.value();
}

QString ToolRegistry::saveFileFilter(const QString &ext) {
  auto it = saveFileFilters.find(ext);
  if (it != saveFileFilters.end()) {
    return *it;
  }

  QString filter;
  auto semi = false;
  for (const auto &name : extensions.keys()) {
    const auto &exts = extensions[name];
    if (exts.contains(ext)) {
      for (const auto &ext : exts) {
        if (semi) {
          filter += ";;";
        }
        semi = true;
        filter += name;
        filter += " (";
        filter += " *.";
        filter += ext;
        filter += ")";
      }
    }
  }

  saveFileFilters[ext] = filter;
  return filter;
}