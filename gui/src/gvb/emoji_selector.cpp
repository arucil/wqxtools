#include "emoji_selector.h"

#include <QApplication>
#include <QClipboard>
#include <QEvent>
#include <QFont>
#include <QFrame>
#include <QGridLayout>
#include <QHBoxLayout>
#include <QScrollArea>
#include <QScrollBar>
#include <QString>
#include <QToolTip>

#include "emoji_label.h"

EmojiSelector::EmojiSelector(QWidget *parent) :
  QWidget(parent, Qt::Tool | Qt::FramelessWindowHint) {
  initUi();
}

void EmojiSelector::initUi() {
  auto scrollArea = new QScrollArea(this);
  scrollArea->setHorizontalScrollBarPolicy(Qt::ScrollBarAlwaysOff);
  scrollArea->setSizeAdjustPolicy(QScrollArea::AdjustToContentsOnFirstShow);
  auto layout = new QHBoxLayout(this);
  layout->setContentsMargins(4, 4, 4, 4);
  layout->addWidget(scrollArea);

  auto container = new QWidget;
  auto containerLayout = new QGridLayout(container);
  auto margins = containerLayout->contentsMargins();
  margins.setRight(
    margins.right() + scrollArea->verticalScrollBar()->sizeHint().width());
  containerLayout->setContentsMargins(margins);

  const int groups[] =
    {79, 53, 36, 37, 54, 44, 30, 23, 16, 21, 17, 13, 41, 24, 39};

  const int COLUMNS = 16;

  char16_t ch = 0xe000;
  int y = 0;
  bool sep = false;
  QFont font("WenQuXing", 12);
  for (const auto g : groups) {
    if (sep) {
      QFrame *line = new QFrame(container);
      line->setFrameShape(QFrame::HLine);
      containerLayout->addWidget(line, y, 0, 1, COLUMNS);
      y++;
    }
    sep = true;
    int x = 0;
    for (int i = 0; i < g; i++) {
      char16_t c = ch + i;
      auto s = QString::fromUtf16(&c, 1);
      auto l = new EmojiLabel(s);
      l->setFont(font);
      l->setToolTip(
        QString("点击复制<br><font face=\"WenQuXing\" size=\"50\">%1</font>")
          .arg(s));
      connect(l, &EmojiLabel::clicked, this, &EmojiSelector::clickedEmoji);
      connect(
        l,
        &EmojiLabel::released,
        this,
        &EmojiSelector::releasedEmoji,
        Qt::QueuedConnection);
      containerLayout->addWidget(l, y, x);
      x++;
      if (x == COLUMNS) {
        y++;
        x = 0;
      }
    }
    ch += g;
    if (x != 0) {
      y++;
      x = 0;
    }
  }

  scrollArea->setWidget(container);
}

void EmojiSelector::changeEvent(QEvent *event) {
  QWidget::changeEvent(event);
  if (event->type() == QEvent::ActivationChange) {
    if (!this->isActiveWindow()) {
      hide();
    }
  }
}

void EmojiSelector::showEvent(QShowEvent *event) {
  QWidget::showEvent(event);
  emit shown();
}

void EmojiSelector::clickedEmoji(QLabel *label) {
  QApplication::clipboard()->setText(label->text());
}

void EmojiSelector::releasedEmoji(QLabel *label) {
  auto pos =
    label->mapToGlobal(QPoint(label->width() / 2, label->height() / 2));
  QToolTip::showText(pos, "已复制", label, QRect(), 500);
}