#include "double_spinbox.h"

DoubleSpinBox::DoubleSpinBox(QWidget *parent) : QDoubleSpinBox(parent) {}

QString DoubleSpinBox::textFromValue(double value) const {
  return QString::number(value);
}