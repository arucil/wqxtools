#include "tool_factory.h"

#include <QString>
#include <map>

using std::optional;

static std::map<QString, std::function<ToolCtor>> toolFactories;

static std::map<QString, std::set<QString>> extensions;

static optional<QString> openFileFilter;

static std::map<QString, QString> saveFileFilters;

optional<std::function<ToolCtor>> ToolFactoryRegistry::get(const QString &ext) {
  auto it = toolFactories.find(ext.toLower());
  if (it == toolFactories.end()) {
    return {};
  } else {
    return it->second;
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
  for (const auto &i : extensions) {
    if (semi) {
      filter += ";;";
    }
    semi = true;
    filter += i.first;
    filter += " (";
    for (auto &ext : i.second) {
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
    return it->second;
  }

  QString filter;
  auto semi = false;
  for (const auto &i : extensions) {
    if (i.second.count(ext)) {
      for (const auto &ext : i.second) {
        if (semi) {
          filter += ";;";
        }
        semi = true;
        filter += i.first;
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