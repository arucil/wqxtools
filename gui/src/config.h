#pragma once

#include <QObject>

class Config: public QObject {
  Q_OBJECT

signals:
  void configChanged();

public:
  static Config &instance();
};