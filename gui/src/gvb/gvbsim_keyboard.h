#pragma once

#include <QWidget>
#include <cstdint>

class QPushButton;
class QString;

class GvbSimKeyboard: public QWidget {
  Q_OBJECT

public:
  GvbSimKeyboard(QWidget *parent = nullptr);
  ~GvbSimKeyboard();

signals:
  void keyDown(std::uint8_t);
  void keyUp(std::uint8_t);

private:
  void initUi();
  QPushButton *makeButton(const QString &, const QString &, std::uint8_t);
};

std::uint8_t qtKeyToWqxKey(int key);