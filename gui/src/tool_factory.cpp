#include "tool_factory.h"

#include <QString>
#include <QMap>

using std::optional;

static QMap<QString, std::function<ToolCtor>> toolFactories;

static QMap<QString, QSet<QString>> extensions;

static optional<QString> openFileFilter;

static QMap<QString, QString> saveFileFilters;

optional<std::function<ToolCtor>> ToolFactoryRegistry::get(const QString &ext) {
  auto it = toolFactories.find(ext.toLower());
  if (it == toolFactories.end()) {
    return {};
  } else {
    return *it;
  }
}

void ToolFactoryRegistry::registerFactory(
  const QString &name,
  const ToolFactory &factory) {
  for (const auto &ext : factory.extensions) {
    toolFactories[ext] = factory.ctor;
  }

  for (const auto &ext : factory.extensions) {
    extensions[name].insert(ext.toLower());
  }
}

const QString &ToolFactoryRegistry::openFileFilter() {
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

QString ToolFactoryRegistry::saveFileFilter(const QString &ext) {
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