#include "gvb_util.h"

#include <QTextStream>

#include "gvbsim_input_dialog.h"

QString arrayBindingName(const api::GvbBinding::Array_Body &array) {
  return arraySubsToString({array.name.data, array.name.len}, array.dimensions);
}

QString arraySubsToString(
  const api::Utf8Str &name,
  const api::Array<std::uint16_t> &subs) {
  QString result;
  QTextStream arr(&result);
  arr << QString::fromUtf8(name.data, name.len);
  arr << '(';
  auto comma = false;
  for (auto sub = subs.data; sub < subs.data + subs.len; sub++) {
    if (comma) {
      arr << ",";
    }
    comma = true;
    arr << *sub;
  }
  arr << ')';
  return result;
}

std::optional<api::GvbValue> inputString(
  QWidget *parent,
  const api::GvbVirtualMachine *vm,
  const QString &title,
  const api::Array<std::uint8_t> &init) {
  api::GvbExecResult res;
  res.tag = api::GvbExecResult::Tag::KeyboardInput;
  res.keyboard_input.prompt.tag = api::Maybe<api::Utf8String>::Tag::Nothing;
  api::GvbKeyboardInputType types[1];
  types[0].tag = api::GvbKeyboardInputType::Tag::String;
  res.keyboard_input.fields.data = types;
  res.keyboard_input.fields.len = 1;

  api::GvbKeyboardInput initial[1];
  initial[0].tag = api::GvbKeyboardInput::Tag::String;
  initial[0].string._0 = init;
  auto initialInput = api::gvb_new_input_array(initial, 1);
  //api::gvb_destroy_value(value);
  GvbSimInputDialog dlg(parent, vm, res.keyboard_input, &initialInput);
  api::gvb_destroy_input_array(initialInput);
  dlg.setWindowTitle(title);
  dlg.setModal(true);
  if (dlg.exec() == QDialog::Rejected) {
    return {};
  }

  api::GvbValue value;
  value.tag = api::GvbValue::Tag::String;
  value.string._0 = dlg.inputData()[0].string._0;

  return value;
}