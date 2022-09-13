#pragma once

#include <QImage>
#include <QRect>
#include <QWidget>

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
  void paintEvent(QPaintEvent *) override;

private:
  void updateColors();

private:
  QImage m_img;
  QRect m_dirtyArea;
};