#include "gvbsim_screen.h"
#include <QPaintEvent>
#include <QPainter>

GvbSimScreen::GvbSimScreen(QWidget *parent, std::uint8_t *graphics_memory)
    : QWidget(parent),
      m_img(graphics_memory, 160, 80, QImage::Format_Mono), m_dirtyArea{
                                                                0, 0, 160, 80} {
  m_img.setColor(0, 0xff'a3'c0'4c);
  m_img.setColor(0, 0xff'3a'3f'2b);
}

GvbSimScreen::~GvbSimScreen() {}

void GvbSimScreen::markDirty(const QRect &area) {
  m_dirtyArea = area;
}

void GvbSimScreen::paintEvent(QPaintEvent *ev) {
  const int SCALE = 3;
  QPainter painter(this);
  painter.scale(SCALE, SCALE);
  QPoint target{m_dirtyArea.x() * SCALE, m_dirtyArea.y() * SCALE};
  painter.drawImage(target, m_img, m_dirtyArea);
}