#include "gvbsim_input_dialog.h"

#include <QDialogButtonBox>
#include <QDoubleSpinBox>
#include <QFormLayout>
#include <QKeyEvent>
#include <QLabel>
#include <QLineEdit>
#include <QPushButton>
#include <QSpinBox>
#include <QString>
#include <QStyle>
#include <QVBoxLayout>
#include <algorithm>
#include <limits>
#include <stdexcept>

#include "gvbsim_window.h"

GvbSimInputDialog::GvbSimInputDialog(
  QWidget *parent,
  const api::GvbVirtualMachine *vm,
  const api::GvbExecResult::KeyboardInput_Body &input) :
  QDialog(parent),
  m_vm(vm),
  m_input(static_cast<int>(input.fields.len)),
  m_validateAll(false),
  m_validatedFields(0),
  m_validateOkFields(0) {
  initUi(input);
  setWindowTitle("输入");
}

void GvbSimInputDialog::initUi(
  const api::GvbExecResult::KeyboardInput_Body &input) {
  setStyleSheet(R"(
    QVBoxLayout QLabel {
      color: hsl(0, 100%, 50%);
    }
    QLabel { background: green; }
    QLineEdit { font: "WenQuXing"; font-size: 12px; }
  )");

  auto layout = new QVBoxLayout(this);
  if (input.prompt.tag == api::Maybe<api::Utf8String>::Tag::Just) {
    layout->addWidget(new QLabel(
      QString::fromUtf8(input.prompt.just._0.data, input.prompt.just._0.len)));
  }

  auto form = new QFormLayout();
  form->setFieldGrowthPolicy(QFormLayout::AllNonFixedFieldsGrow);
  form->setLabelAlignment(Qt::AlignTop);
  layout->addLayout(form);

  QWidget *lastField = nullptr;
  for (size_t i = 0; i < input.fields.len; i++) {
    const auto &field = input.fields.data[i];
    QWidget *fieldInput;
    // [x] tab order
    // press enter go to next field
    // [x] ctrl+enter finish
    // [x] esc quit
    // enter check current field
    // OK check all fields
    switch (field.tag) {
      case api::GvbKeyboardInputType::Tag::Integer: {
        m_input[i] = InputField {std::in_place_index<0>, 0};
        auto input = new QSpinBox();
        fieldInput = input;
        input->setMinimum(-32768);
        input->setMaximum(32767);
        input->setToolTip("范围：-32768 ~ 32767");
        connect(
          this,
          &GvbSimInputDialog::validateAll,
          input,
          &QSpinBox::editingFinished);
        connect(input, &QSpinBox::editingFinished, this, [i, input, this] {
          m_input[i] = static_cast<std::int16_t>(input->value());
          emit fieldValidated(true);
        });
        form->addRow("整数", input);
        break;
      }
      case api::GvbKeyboardInputType::Tag::Real: {
        m_input[i] = InputField {std::in_place_index<1>, api::GvbReal {0.0}};
        auto input = new QDoubleSpinBox();
        fieldInput = input;
        input->setMinimum(-1.7e38);
        input->setMaximum(1.7e38);
        input->setToolTip("范围：-1.7E+38 ~ +1.7E+38");
        connect(
          this,
          &GvbSimInputDialog::validateAll,
          input,
          &QDoubleSpinBox::editingFinished);
        connect(input, &QSpinBox::editingFinished, this, [i, input, this] {
          m_input[i] = api::GvbReal {input->value()};
          emit fieldValidated(true);
        });
        form->addRow("实数", input);
        break;
      }
      case api::GvbKeyboardInputType::Tag::String: {
        m_input[i] =
          InputField {std::in_place_index<2>, ByteString {nullptr, 0}};
        auto layout = new QVBoxLayout();
        auto input = new QLineEdit();
        layout->addWidget(input);
        fieldInput = input;
        auto msg = new QLabel(" ");
        layout->addWidget(msg);
        connect(
          this,
          &GvbSimInputDialog::validateAll,
          input,
          &QLineEdit::editingFinished);
        connect(
          input,
          &QLineEdit::editingFinished,
          this,
          [i, input, msg, this] {
            auto s = input->text();
            auto bstr = api::utf16_to_byte_string_lossy(
              {s.utf16(), static_cast<size_t>(s.size())});
            if (bstr.len > 255) {
              msg->setText(tr("字符串长度为 %1，超出上限 255").arg(bstr.len));
              emit fieldValidated(false);
            } else {
              auto old = std::get<2>(m_input[i]);
              m_input[i] = InputField {std::in_place_index<2>, bstr};
              bstr = old;
              emit fieldValidated(true);
            }
            api::destroy_byte_string(bstr);
          });
        form->addRow("字符串", layout);
        break;
      }
      case api::GvbKeyboardInputType::Tag::Func: {
        m_input[i] = InputField {std::in_place_index<3>, nullptr};
        auto layout = new QVBoxLayout();
        auto input = new QLineEdit();
        layout->addWidget(input);
        fieldInput = input;
        auto msg = new QLabel(" ");
        layout->addWidget(msg);
        connect(
          this,
          &GvbSimInputDialog::validateAll,
          input,
          &QLineEdit::editingFinished);
        connect(
          input,
          &QLineEdit::editingFinished,
          this,
          [i, input, msg, this] {
            auto s = input->text();
            auto result = api::gvb_compile_fn_body(
              m_vm,
              {s.utf16(), static_cast<size_t>(s.size())});
            auto error = false;
            api::Utf8String firstErrorMsg;
            size_t firstErrorStart = std::numeric_limits<size_t>::max();
            for (auto p = result.diagnostics.data;
                 p < result.diagnostics.data + result.diagnostics.len;
                 p++) {
              if (
                p->severity == api::GvbSeverity::Error
                && (!error || p->start < firstErrorStart)) {
                firstErrorMsg = p->message;
                firstErrorStart = p->start;
              }
            }
            if (error) {
              msg->setText(
                tr("错误：%1")
                  .arg(
                    QString::fromUtf8(firstErrorMsg.data, firstErrorMsg.len)));
              emit fieldValidated(false);
            } else {
              auto old = std::get<3>(m_input[i]);
              m_input[i] = InputField {std::in_place_index<3>, result.body};
              result.body = old;
              emit fieldValidated(true);
            }
            api::gvb_destroy_fn_body(result.body);
            api::gvb_destroy_string_diagnostic_array(result.diagnostics);
          });
        form->addRow(
          tr("函数 %1(%2) =")
            .arg(QString::fromUtf8(field.func.name.data, field.func.name.len))
            .arg(
              QString::fromUtf8(field.func.param.data, field.func.param.len)),
          layout);
        break;
      }
      default:
        throw std::logic_error("invalid keyboard input type");
    }

    if (lastField) {
      QWidget::setTabOrder(lastField, fieldInput);
    } else {
      lastField = fieldInput;
      lastField->focusWidget();
    }
  }

  auto confirm =
    new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
  confirm->button(QDialogButtonBox::Ok)->setShortcut(Qt::CTRL | Qt::Key_Return);
  confirm->button(QDialogButtonBox::Cancel)->setShortcut(Qt::Key_Escape);
  connect(confirm, &QDialogButtonBox::rejected, this, &QDialog::reject);
  connect(
    confirm,
    &QDialogButtonBox::accepted,
    this,
    &GvbSimInputDialog::startValidateAll);

  layout->addWidget(confirm);
}

QVector<api::GvbKeyboardInput> GvbSimInputDialog::inputData() {
  QVector<api::GvbKeyboardInput> result;
  for (const auto &field : m_input) {
    api::GvbKeyboardInput i;
    if (auto n = std::get_if<std::int16_t>(&field)) {
      i.tag = api::GvbKeyboardInput::Tag::Integer;
      i.integer._0 = *n;
      result.push_back(i);
    } else if (auto n = std::get_if<api::GvbReal>(&field)) {
      i.tag = api::GvbKeyboardInput::Tag::Real;
      i.real._0 = *n;
      result.push_back(i);
    } else if (auto s = std::get_if<2>(&field)) {
      i.tag = api::GvbKeyboardInput::Tag::String;
      i.string._0 = *s;
      result.push_back(i);
    } else if (auto f = std::get_if<3>(&field)) {
      i.tag = api::GvbKeyboardInput::Tag::Func;
      i.func._0 = *f;
      result.push_back(i);
    }
  }
  return result;
}

void GvbSimInputDialog::startValidateAll() {
  m_validateAll = true;
  m_validatedFields = 0;
  m_validateOkFields = 0;
  emit validateAll();
}

void GvbSimInputDialog::fieldValidated(bool ok) {
  if (!m_validateAll) {
    return;
  }

  if (ok) {
    m_validateOkFields++;
  }

  if (++m_validatedFields == static_cast<size_t>(m_input.size())) {
    endValidateAll();
  }
}

void GvbSimInputDialog::endValidateAll() {
  if (m_validateOkFields == m_validatedFields) {
    emit accept();
  }
  m_validateAll = false;
  m_validatedFields = 0;
  m_validateOkFields = 0;
}