#pragma once

#include <QObject>
#include <QString>
#include <QHash>
#include "syntax_style.h"

class Config: public QObject {
  Q_OBJECT

signals:
  void configChanged();

public:
  const SyntaxStyle *getStyle(const QString &) const;
  void addStyle(const QString &name, const SyntaxStyle &style);

public:
  static Config &instance();

private:
  QHash<QString, SyntaxStyle> m_styles;
};