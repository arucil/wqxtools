#include "value.h"

BoolValue::BoolValue() : m_value(false) {}

bool BoolValue::value() const {
  return m_value;
}

void BoolValue::setValue(bool newValue) {
  if (newValue != m_value) {
    m_value = newValue;
    emit changed(newValue);
  }
}

const QString &StrValue::value() const {
  return m_value;
}

QString &StrValue::value() {
  return m_value;
}

void StrValue::setValue(const QString &newValue) {
  if (newValue != m_value) {
    m_value = newValue;
    emit changed(newValue);
  }
}
