#pragma once

#include <QImage>
#include <QWidget>
#include <QRect>
#include <cstdint>

class QPaintEvent;

class GvbSimScreen : public QWidget {
  Q_OBJECT

public:
  GvbSimScreen(QWidget *parent, std::uint8_t *);
  ~GvbSimScreen();

public slots:
  void markDirty(const QRect &);

protected:
  void paintEvent(QPaintEvent *) Q_DECL_OVERRIDE;

private:
  QImage m_img;
  QRect m_dirtyArea;
};