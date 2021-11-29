#include "gvbsim_keyboard.h"

#include <QFont>
#include <QGridLayout>
#include <QHash>
#include <QPushButton>
#include <QString>
#include <QTimer>

static const QHash<int, std::uint8_t> KEY_MAPPINGS {
  {Qt::Key_F1, 28},        {Qt::Key_F2, 29},       {Qt::Key_F3, 30},
  {Qt::Key_F4, 31},

  {Qt::Key_O, 111},        {Qt::Key_L, 108},       {Qt::Key_Up, 20},
  {Qt::Key_Down, 21},      {Qt::Key_P, 112},       {Qt::Key_Return, 13},
  {Qt::Key_PageDown, 14},  {Qt::Key_Right, 22},

  {Qt::Key_Q, 113},        {Qt::Key_W, 119},       {Qt::Key_E, 101},
  {Qt::Key_R, 114},        {Qt::Key_T, 116},       {Qt::Key_Y, 121},
  {Qt::Key_U, 117},        {Qt::Key_I, 105},

  {Qt::Key_A, 97},         {Qt::Key_S, 115},       {Qt::Key_D, 100},
  {Qt::Key_F, 102},        {Qt::Key_G, 103},       {Qt::Key_H, 104},
  {Qt::Key_J, 106},        {Qt::Key_K, 107},

  {Qt::Key_Z, 122},        {Qt::Key_X, 120},       {Qt::Key_C, 99},
  {Qt::Key_V, 118},        {Qt::Key_B, 98},        {Qt::Key_N, 110},
  {Qt::Key_M, 109},        {Qt::Key_PageUp, 19},

  {Qt::Key_Control, 25},  // [Ctrl] -> [求助]
  {Qt::Key_Shift, 26},     {Qt::Key_CapsLock, 18}, {Qt::Key_Escape, 27},
  {Qt::Key_0, 48},         {Qt::Key_Period, 46},   {Qt::Key_Space, 32},
  {Qt::Key_Left, 23},

  {Qt::Key_1, 98},         {Qt::Key_2, 110},       {Qt::Key_3, 109},
  {Qt::Key_4, 103},        {Qt::Key_5, 104},       {Qt::Key_6, 106},
  {Qt::Key_7, 116},        {Qt::Key_8, 121},       {Qt::Key_9, 117},

  {Qt::Key_Enter, 13},  // Numpad Enter

  {Qt::Key_AsciiTilde, 18}  // [~] -> [输入法]
};

std::uint8_t qtKeyToWqxKey(int key) {
  return KEY_MAPPINGS[key];
}

GvbSimKeyboard::GvbSimKeyboard(QWidget *parent) : QWidget(parent) {
  initUi();

  QTimer::singleShot(40, this, [this] {
    QFont f;
    f.setPointSize(f.pointSize() - 1);
    for (auto btn : findChildren<QPushButton *>()) {
      btn->setFont(f);
    }
  });
}

GvbSimKeyboard::~GvbSimKeyboard() {}

void GvbSimKeyboard::initUi() {
  auto layout = new QGridLayout(this);
  layout->addWidget(makeButton("F1", "F1", 28), 0, 6);
  layout->addWidget(makeButton("F2", "F2", 29), 0, 7);
  layout->addWidget(makeButton("F3", "F3", 30), 0, 8);
  layout->addWidget(makeButton("F4", "F4", 31), 0, 9);

  layout->addWidget(makeButton("Q", "Q", 113), 1, 0);
  layout->addWidget(makeButton("W", "W", 119), 1, 1);
  layout->addWidget(makeButton("E", "E", 101), 1, 2);
  layout->addWidget(makeButton("R", "R", 114), 1, 3);
  layout->addWidget(makeButton("T➐", "T / 7", 116), 1, 4);
  layout->addWidget(makeButton("Y➑", "Y / 8", 121), 1, 5);
  layout->addWidget(makeButton("U➒", "U / 9", 117), 1, 6);
  layout->addWidget(makeButton("I", "I", 105), 1, 7);
  layout->addWidget(makeButton("O", "O", 111), 1, 8);
  layout->addWidget(makeButton("P", "P", 112), 1, 9);

  layout->addWidget(makeButton("A", "A", 97), 2, 0);
  layout->addWidget(makeButton("S", "S", 115), 2, 1);
  layout->addWidget(makeButton("D", "D", 100), 2, 2);
  layout->addWidget(makeButton("F", "F", 102), 2, 3);
  layout->addWidget(makeButton("G➍", "G / 4", 103), 2, 4);
  layout->addWidget(makeButton("H➎", "H / 5", 104), 2, 5);
  layout->addWidget(makeButton("J➏", "J / 6", 106), 2, 6);
  layout->addWidget(makeButton("K", "K", 107), 2, 7);
  layout->addWidget(makeButton("L", "L", 108), 2, 8);
  layout->addWidget(makeButton("输入", "Enter", 13), 2, 9);

  layout->addWidget(makeButton("Z", "Z", 122), 3, 0);
  layout->addWidget(makeButton("X", "X", 120), 3, 1);
  layout->addWidget(makeButton("C", "C", 120), 3, 2);
  layout->addWidget(makeButton("V", "V", 118), 3, 3);
  layout->addWidget(makeButton("B➊", "B / 1", 98), 3, 4);
  layout->addWidget(makeButton("N➋", "N / 2", 110), 3, 5);
  layout->addWidget(makeButton("M➌", "M / 3", 109), 3, 6);
  layout->addWidget(makeButton("上翻页", "PageUp", 19), 3, 7);
  layout->addWidget(makeButton("↑", "上", 20), 3, 8);
  layout->addWidget(makeButton("下翻页", "PageDown", 14), 3, 9);

  layout->addWidget(makeButton("求助", "Ctrl", 25), 4, 0);
  layout->addWidget(makeButton("中英数", "Shift", 26), 4, 1);
  layout->addWidget(makeButton("输入法", "CapsLock / ~", 18), 4, 2);
  layout->addWidget(makeButton("跳出", "Esc", 27), 4, 3);
  layout->addWidget(makeButton("符号⓿", "0", 48), 4, 4);
  layout->addWidget(makeButton(".", ".", 46), 4, 5);
  layout->addWidget(makeButton("空格", "空格", 32), 4, 6);
  layout->addWidget(makeButton("←", "左", 23), 4, 7);
  layout->addWidget(makeButton("↓", "下", 21), 4, 8);
  layout->addWidget(makeButton("→", "右", 22), 4, 9);

  layout->setHorizontalSpacing(3);
  layout->setVerticalSpacing(4);
  layout->setMargin(0);

  setStyleSheet(R"(
    QPushButton {
      width: 40px;
      height: 22px;
      border-radius: 4px;
      border: 1px solid #bbb;
      background: hsla(70, 80%, 50%, 5%);
    }
    QPushButton:hover {
      background: hsla(70, 80%, 50%, 25%);
    }
    QPushButton:pressed {
      background: hsla(70, 80%, 50%, 50%);
    }
  )");
}

QPushButton *GvbSimKeyboard::makeButton(
  const QString &text,
  const QString &tooltip,
  std::uint8_t key) {
  auto btn = new QPushButton(text, this);
  btn->setToolTip(tooltip);
  btn->setFocusPolicy(Qt::NoFocus);
  connect(btn, &QPushButton::pressed, this, [this, key] { emit keyDown(key); });
  connect(btn, &QPushButton::released, this, [this, key] { emit keyUp(key); });
  return btn;
}