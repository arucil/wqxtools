#pragma once

#include <QLabel>

class QMouseEvent;

class EmojiLabel: public QLabel {
  Q_OBJECT
public:
  EmojiLabel(const QString &, QWidget *parent = nullptr);

signals:
  void clicked(QLabel *);
  void released(QLabel *);

protected:
  void mousePressEvent(QMouseEvent *) override;
  void mouseReleaseEvent(QMouseEvent *) override;
};