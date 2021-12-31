#pragma once

#include <QObject>
#include <QString>

class BoolValue: public QObject {
  Q_OBJECT

public:
  BoolValue();

  bool value() const;

signals:
  void changed(bool);

public slots:
  void setValue(bool);

private:
  bool m_value;
};

class StrValue: public QObject {
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

class SizeValue: public QObject {
  Q_OBJECT

public:
  SizeValue();

  size_t value() const;

signals:
  void changed(size_t);

public slots:
  void setValue(size_t);

private:
  size_t m_value;
};