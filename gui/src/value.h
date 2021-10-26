#pragma once

#include <QObject>

class BoolValue : public QObject {
  Q_OBJECT

public:
  bool value() const;

signals:
  void changed(bool);

public slots:
  void setValue(bool);

private:
  bool m_value;
};