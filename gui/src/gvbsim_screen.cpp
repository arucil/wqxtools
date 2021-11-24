#include "gvbsim_screen.h"
#include <QPaintEvent>
#include <QPainter>

const int SCALE = 2;

GvbSimScreen::GvbSimScreen(QWidget *parent) : QWidget(parent) {
  setFixedSize({160 * SCALE, 80 * SCALE});
}

GvbSimScreen::~GvbSimScreen() {}

void GvbSimScreen::setImageData(const std::uint8_t *data) {
  if (data == nullptr) {
    m_img = QImage(160, 80, QImage::Format_Mono);
  } else {
    // the const variant does not work
    m_img = QImage(const_cast<uint8_t *>(data), 160, 80, QImage::Format_Mono);
  }
  m_img.setColor(0, 0xff'79'95'78);
  m_img.setColor(1, 0xff'3e'46'82);
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
    painter.drawImage(QPoint{rect.x(), rect.y()}, m_img, rect);
  } else {
    // not triggered manually
    painter.drawImage(m_img.rect(), m_img);
  }
}