#pragma once

#include <QLabel>

class QString;
class QTimerEvent;
class QPaintEvent;
class QPropertyAnimation;
class QGraphicsOpacityEffect;

class Toast : public QLabel {
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
  QGraphicsOpacityEffect *m_opacityEffect;
  QPropertyAnimation *m_fadeIn;
  QPropertyAnimation *m_fadeOut;
  int m_timer;
  int m_delay;
};