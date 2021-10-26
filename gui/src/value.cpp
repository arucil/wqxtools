#include "value.h"

bool BoolValue::value() const { return m_value; }

void BoolValue::setValue(bool newValue) {
  if (newValue != m_value) {
    m_value = newValue;
    emit(changed(newValue));
  }
}