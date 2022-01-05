#pragma once

#include <QObject>
#include <optional>
#include "syntax_style.h"

class Config: public QObject {
  Q_OBJECT

signals:
  void configChanged();
  void styleChanged(const SyntaxStyle *);

public:
  const SyntaxStyle *getStyle() const;
  void setStyle(std::optional<SyntaxStyle>);

public:
  static Config &instance();

private:
  std::optional<SyntaxStyle> m_style;
};