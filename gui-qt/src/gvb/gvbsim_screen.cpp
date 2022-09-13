#include "gvbsim_screen.h"

#include <QPainter>

#include "../config.h"
#include "api.h"

GvbSimScreen::GvbSimScreen(QWidget *parent) : QWidget(parent) {
  connect(
    Config::instance(),
    &Config::configChanged,
    this,
    &GvbSimScreen::configChanged);
  configChanged();
}

GvbSimScreen::~GvbSimScreen() {}

void GvbSimScreen::configChanged() {
  const auto scale = static_cast<int>(api::config()->gvb.simulator.pixel_scale);
  setFixedSize({160 * scale, 80 * scale});
  updateColors();
  update();
}

void GvbSimScreen::updateColors() {
  const auto &cfg = api::config()->gvb.simulator;
  m_img.setColor(0, 0xff000000 | cfg.background);
  m_img.setColor(1, 0xff000000 | cfg.foreground);
}

void GvbSimScreen::setImageData(const std::uint8_t *data) {
  if (data == nullptr) {
    m_img = QImage(160, 80, QImage::Format_Mono);
  } else {
    // the const variant does not work
    m_img = QImage(const_cast<uint8_t *>(data), 160, 80, QImage::Format_Mono);
  }
  updateColors();
}

void GvbSimScreen::markDirty(const QRect &area) {
  m_dirtyArea = area;
}

void GvbSimScreen::paintEvent(QPaintEvent *) {
  QPainter painter(this);
  const auto scale = api::config()->gvb.simulator.pixel_scale;
  painter.scale(scale, scale);
  if (!m_dirtyArea.isNull()) {
    auto rect = m_dirtyArea;
    m_dirtyArea = QRect();
    painter.drawImage(QPoint {rect.x(), rect.y()}, m_img, rect);
  } else {
    // not triggered manually
    painter.drawImage(m_img.rect(), m_img);
  }
}