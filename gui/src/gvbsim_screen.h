#pragma once

#include <QImage>
#include <QWidget>
#include <QRect>
#include <cstdint>
#include <optional>

class QPaintEvent;

class GvbSimScreen : public QWidget {
  Q_OBJECT

public:
  GvbSimScreen(QWidget *parent);
  ~GvbSimScreen();

  void setImageData(const std::uint8_t *);

public slots:
  void markDirty(const QRect &);

protected:
  void paintEvent(QPaintEvent *) Q_DECL_OVERRIDE;

private:
  QImage m_img;
  std::optional<QRect> m_dirtyArea;
};