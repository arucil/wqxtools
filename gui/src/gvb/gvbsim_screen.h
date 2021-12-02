#pragma once

#include <QImage>
#include <QRect>
#include <QWidget>
#include <cstdint>
#include <optional>

class QPaintEvent;

class GvbSimScreen: public QWidget {
  Q_OBJECT

public:
  GvbSimScreen(QWidget *parent);
  ~GvbSimScreen();

  void setImageData(const std::uint8_t *);

public slots:
  void markDirty(const QRect &);
  void configChanged();

protected:
  void paintEvent(QPaintEvent *) Q_DECL_OVERRIDE;

private:
  void updateColors();

private:
  QImage m_img;
  std::optional<QRect> m_dirtyArea;
};