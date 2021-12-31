#pragma once

#include <QWidget>
#include <cstdint>

class QPushButton;
class QString;

using std::uint8_t;

class GvbSimKeyboard: public QWidget {
  Q_OBJECT

public:
  GvbSimKeyboard(QWidget *parent = nullptr);
  ~GvbSimKeyboard();

signals:
  void keyDown(uint8_t);
  void keyUp(uint8_t);

private:
  void initUi();
  QPushButton *makeButton(const QString &, const QString &, uint8_t);
};

uint8_t qtKeyToWqxKey(int key);