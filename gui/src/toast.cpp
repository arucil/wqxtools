#include "toast.h"

#include <QApplication>
#include <QGraphicsOpacityEffect>
#include <QHBoxLayout>
#include <QLabel>
#include <QPainter>
#include <QPropertyAnimation>
#include <QScreen>

Toast::Toast(QWidget *parent) :
  QWidget(parent, Qt::FramelessWindowHint | Qt::Tool),
  m_label(new QLabel(this)),
  m_opacityEffect(new QGraphicsOpacityEffect(this)),
  m_fadeIn(new QPropertyAnimation(m_opacityEffect, "opacity", this)),
  m_fadeOut(new QPropertyAnimation(m_opacityEffect, "opacity", this)),
  m_timer(0) {
  setAttribute(Qt::WA_TranslucentBackground);
  setGraphicsEffect(m_opacityEffect);

  m_fadeIn->setEndValue(1);
  m_fadeIn->setDuration(100);
  connect(
    m_fadeIn,
    &QPropertyAnimation::finished,
    this,
    &Toast::startFadeOutTimer);

  m_fadeOut->setStartValue(1);
  m_fadeOut->setEndValue(0);
  m_fadeOut->setDuration(250);
  connect(m_fadeOut, &QPropertyAnimation::finished, this, &QWidget::hide);

  auto layout = new QHBoxLayout(this);
  layout->addWidget(m_label);

  hide();
}

void Toast::showText(const QString &text, int ms) {
  m_label->setText(text);
  adjustSize();
  m_delay = ms;
  if (m_timer == 0) {
    show();
    m_fadeOut->stop();
    m_fadeIn->setStartValue(m_opacityEffect->opacity());
    m_fadeIn->start();
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
  move(topCenter - QPoint(width() / 2, height() / 2));
}

void Toast::timerEvent(QTimerEvent *) {
  killTimer(m_timer);
  m_timer = 0;

  m_fadeOut->start();
}

void Toast::startFadeOutTimer() {
  m_timer = startTimer(m_delay);
}

void Toast::paintEvent(QPaintEvent *) {
  QPainter p(this);
  p.setBrush(palette().brush(QPalette::Inactive, QPalette::ToolTipBase));
  p.setPen(palette().color(QPalette::Inactive, QPalette::ToolTipText));
  p.drawRect(rect() - QMargins {1, 1, 1, 1});
}