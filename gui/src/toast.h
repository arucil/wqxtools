#pragma once

#include <QFrame>

class QString;
class QTimerEvent;
class QPaintEvent;
class QLabel;
class QPropertyAnimation;
class QGraphicsOpacityEffect;

class Toast : public QFrame {
  Q_OBJECT

public:
  Toast(QWidget *parent = nullptr);

  void showText(const QString &, int ms);

protected:
  void timerEvent(QTimerEvent *) override;
  void paintEvent(QPaintEvent *) override;

private:
  void hideToast();
  void startFadeOutTimer();

private:
  QLabel *m_label;
  QGraphicsOpacityEffect *m_opacityEffect;
  QPropertyAnimation *m_fade;
  int m_timer;
  int m_delay;
};