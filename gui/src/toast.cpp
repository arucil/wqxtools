#include "toast.h"

#include <QApplication>
#include <QGraphicsOpacityEffect>
#include <QPainter>
#include <QPropertyAnimation>
#include <QScreen>
#include <QStyleOptionFrame>
#include <QStylePainter>
#include <QToolTip>

Toast::Toast(QWidget *parent) :
  QLabel(parent),
  m_opacityEffect(new QGraphicsOpacityEffect(this)),
  m_fadeIn(new QPropertyAnimation(m_opacityEffect, "opacity", this)),
  m_fadeOut(new QPropertyAnimation(m_opacityEffect, "opacity", this)),
  m_timer(0) {
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

  setForegroundRole(QPalette::ToolTipText);
  setBackgroundRole(QPalette::ToolTipBase);
  setPalette(QToolTip::palette());
  setMargin(
    5 + style()->pixelMetric(QStyle::PM_ToolTipLabelFrameWidth, nullptr, this));

  hide();
}

void Toast::showText(const QString &text, int ms) {
  setText(text);
  adjustSize();
  m_delay = ms;
  if (m_timer == 0) {
    show();
    raise();
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
    topCenter = QPoint(parent->width() / 2, parent->height() * 4 / 5);
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

void Toast::paintEvent(QPaintEvent *ev) {
  QStylePainter p(this);
  QStyleOptionFrame opt;
  opt.initFrom(this);
  p.drawPrimitive(QStyle::PE_PanelTipLabel, opt);
  p.end();

  QLabel::paintEvent(ev);
}