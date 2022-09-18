#include "emoji_label.h"

#include <QMouseEvent>

EmojiLabel::EmojiLabel(const QString &text, QWidget *parent) :
  QLabel(text, parent) {
}

void EmojiLabel::mousePressEvent(QMouseEvent *event) {
  QLabel::mousePressEvent(event);
  if (
    event->button() == Qt::LeftButton && event->modifiers() == Qt::NoModifier) {
    emit clicked(this);
  }
}

void EmojiLabel::mouseReleaseEvent(QMouseEvent *event) {
  QLabel::mouseReleaseEvent(event);
  if (
    event->button() == Qt::LeftButton && event->modifiers() == Qt::NoModifier) {
    emit released(this);
  }
}