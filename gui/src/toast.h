#pragma once

#include <QWidget>

class QString;
class QTimerEvent;
class QPaintEvent;
class QLabel;
class QPropertyAnimation;
class QGraphicsOpacityEffect;

class Toast : public QWidget {
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
  QPropertyAnimation *m_fadeIn;
  QPropertyAnimation *m_fadeOut;
  int m_timer;
  int m_delay;
};