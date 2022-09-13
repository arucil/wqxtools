#pragma once

#include <QDoubleSpinBox>

class DoubleSpinBox : public QDoubleSpinBox {
  Q_OBJECT
public:
  DoubleSpinBox(QWidget * = nullptr);

protected:
  QString textFromValue(double value) const override;
};