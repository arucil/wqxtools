#include "gvbsim_input_dialog.h"

#include <QFormLayout>
#include <QLabel>
#include <QSpinBox>
#include <QString>
#include <QVBoxLayout>

GvbSimInputDialog::GvbSimInputDialog(
  QWidget *parent,
  const api::GvbExecResult::KeyboardInput_Body &input) {
  initUi(input);
  setWindowTitle("输入");
}

void GvbSimInputDialog::initUi(
  const api::GvbExecResult::KeyboardInput_Body &input) {
  auto form = new QFormLayout();

  for (size_t i = 0; i < input.fields.len; i++) {
    const auto &field = input.fields.data[i];
    switch (field.tag) {
      case api::GvbKeyboardInputType::Tag::Integer: {
        auto fieldLayout = new QVBoxLayout();
        auto input = new QSpinBox();
        input->setMinimum(-32768);
        input->setMaximum(32767);
        fieldLayout->addWidget(input);
        auto msg = new QLabel(" ");
        fieldLayout->addWidget(msg);
        form->addRow("整数", fieldLayout);
      }
    }
  }

  if (input.prompt.tag == api::Maybe<api::Utf8String>::Tag::Just) {
    auto layout = new QVBoxLayout();
    layout->addWidget(new QLabel(
      QString::fromUtf8(input.prompt.just._0.data, input.prompt.just._0.len)));
    layout->addLayout(form);
    setLayout(layout);
  } else {
    setLayout(form);
  }
}