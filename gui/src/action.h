#pragma once

#include <QAction>

class QEvent;

class Action : public QAction {
  Q_OBJECT

public:
  explicit Action(QObject *parent = nullptr);
  explicit Action(const QString &text, QObject *parent = nullptr);
  explicit Action(
      const QIcon &icon, const QString &text, QObject *parent = nullptr);

signals:
  void enabledChanged(bool);

protected:
  bool event(QEvent *) Q_DECL_OVERRIDE;
};