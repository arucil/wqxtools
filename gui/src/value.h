#pragma once

#include <QObject>
#include <QString>

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

class StrValue : public QObject {
  Q_OBJECT

public:
  const QString &value() const;
  QString &value();

signals:
  void changed(const QString &);

public slots:
  void setValue(const QString &);

private:
  QString m_value;
};