#include "config.h"

#include <utility>

static Config config;

Config &Config::instance() {
  return config;
}

const SyntaxStyle *Config::getStyle() const {
  if (m_style.has_value()) {
    return &m_style.value();
  } else {
    return nullptr;
  }
}

void Config::setStyle(std::optional<SyntaxStyle> style) {
  m_style = std::move(style);
  emit styleChanged(getStyle());
}