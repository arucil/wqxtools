#include "config.h"

static Config config;

Config &Config::instance() {
  return config;
}

const SyntaxStyle *Config::getStyle(const QString &name) const {
  auto it = m_styles.constFind(name);
  if (it == m_styles.constEnd()) {
    return nullptr;
  } else {
    return &*it;
  }
}

void Config::addStyle(const QString &name, const SyntaxStyle &style) {
  m_styles.insert(name, style);
}