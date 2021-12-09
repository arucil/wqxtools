#include "gvb_util.h"
#include <QTextStream>

QString array_binding_name(const api::GvbBinding::Array_Body &array) {
  QString result;
  QTextStream arr(&result);
  arr << QString::fromUtf8(array.name.data, array.name.len);
  arr << '(';
  auto dimensions = array.dimensions;
  auto comma = false;
  for (auto sub = dimensions.data; sub < dimensions.data + dimensions.len;
       sub++) {
    if (comma) {
      arr << ",";
    }
    comma = true;
    arr << *sub;
  }
  arr << ')';
  return result;
}