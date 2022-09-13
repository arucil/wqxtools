#pragma once

#include <QDialog>
#include <cstdint>

using std::uint16_t;

class RelabelDialog : public QDialog {
  Q_OBJECT

public:
  RelabelDialog(QWidget *parent = nullptr);

signals:
  void relabel(uint16_t start, uint16_t inc);

private:
  void initUi();
};