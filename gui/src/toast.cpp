#include "toast.h"

#include <QApplication>
#include <QDebug>
#include <QGraphicsOpacityEffect>
#include <QHBoxLayout>
#include <QLabel>
#include <QPainter>
#include <QPropertyAnimation>
#include <QScreen>

Toast::Toast(QWidget *parent) :
  QFrame(parent, Qt::FramelessWindowHint | Qt::Tool),
  m_label(new QLabel(this)),
  m_opacityEffect(new QGraphicsOpacityEffect(this)),
  m_fade(new QPropertyAnimation(m_opacityEffect, "opacity", this)),
  m_timer(0) {
  setAttribute(Qt::WA_TranslucentBackground);
  setGraphicsEffect(m_opacityEffect);
  setFrameStyle(QFrame::Box);

  m_fade->setEndValue(1);

  auto layout = new QHBoxLayout(this);
  layout->setContentsMargins(0, 0, 0, 0);

  m_label->setContentsMargins(15, 8, 15, 8);
  layout->addWidget(m_label);

  hide();
}

void Toast::showText(const QString &text, int ms) {
  m_label->setText(text);
  m_delay = ms;
  if (m_timer == 0) {
    show();
    m_fade->stop();
    disconnect(m_fade, &QPropertyAnimation::finished, this, &QWidget::hide);
    m_fade->setDirection(QAbstractAnimation::Forward);
    m_fade->setStartValue(m_opacityEffect->opacity());
    m_fade->setDuration(100);
    connect(
      m_fade,
      &QPropertyAnimation::finished,
      this,
      &Toast::startFadeOutTimer);
    m_fade->start();
  } else {
    killTimer(m_timer);
    m_timer = 0;
    startFadeOutTimer();
  }
  QPoint topCenter;
  if (auto parent = parentWidget()) {
    QPoint offset(parent->width() / 2, parent->height() * 4 / 5);
    topCenter = parent->mapToGlobal(QPoint()) + offset;
  } else {
    auto g = qApp->primaryScreen()->geometry();
    topCenter = g.topLeft() + QPoint(g.width() / 2, g.height() * 4 / 5);
  }
  auto size = sizeHint();
  move(topCenter - QPoint(size.width() / 2, size.height() / 2));
}

void Toast::timerEvent(QTimerEvent *) {
  killTimer(m_timer);
  m_timer = 0;

  m_fade->setDuration(250);
  m_fade->setStartValue(0);
  m_fade->setDirection(QAbstractAnimation::Backward);
  m_fade->start();
  disconnect(
    m_fade,
    &QPropertyAnimation::finished,
    this,
    &Toast::startFadeOutTimer);
  connect(m_fade, &QPropertyAnimation::finished, this, &QWidget::hide);
}

void Toast::startFadeOutTimer() {
  m_timer = startTimer(m_delay);
}

void Toast::paintEvent(QPaintEvent *) {
  QPainter painter(this);
  painter.setBrush(palette().window());
  painter.drawRect(0, 0, width(), height());
}