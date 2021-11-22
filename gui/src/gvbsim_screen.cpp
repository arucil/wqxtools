#include "gvbsim_screen.h"
#include <QPaintEvent>
#include <QPainter>

const int SCALE = 3;

GvbSimScreen::GvbSimScreen(QWidget *parent) : QWidget(parent) {
  m_img.setColor(0, 0xff'a3'c0'4c);
  m_img.setColor(0, 0xff'3a'3f'2b);
  setFixedSize({160 * SCALE, 80 * SCALE});
}

GvbSimScreen::~GvbSimScreen() {}

void GvbSimScreen::setImageData(const std::uint8_t *data) {
  m_img = {data, 160, 80, QImage::Format_Mono};
}

void GvbSimScreen::markDirty(const QRect &area) {
  m_dirtyArea = area;
}

void GvbSimScreen::paintEvent(QPaintEvent *) {
  QPainter painter(this);
  painter.scale(SCALE, SCALE);
  if (m_dirtyArea.has_value()) {
    auto rect = m_dirtyArea.value();
    m_dirtyArea.reset();
    painter.drawImage(QPoint{rect.x() * SCALE, rect.y() * SCALE}, m_img, rect);
  } else {
    // not triggered manually
    painter.drawImage(QPoint{}, m_img);
  }
}