#include "gvbsim_input_dialog.h"

#include <QDialogButtonBox>
#include <QFormLayout>
#include <QFrame>
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

#include "double_spinbox.h"
#include "gvbsim_window.h"

using std::get_if;
using std::in_place_index;
using std::int16_t;

GvbSimInputDialog::GvbSimInputDialog(
  QWidget *parent,
  const api::GvbVirtualMachine *vm,
  const api::GvbExecResult::KeyboardInput_Body &input,
  const api::Array<api::GvbKeyboardInput> *initial) :
  QDialog(parent),
  m_vm(vm),
  m_input(static_cast<int>(input.fields.len)),
  m_validateAll(false),
  m_validatedFields(0),
  m_validateOkFields(0),
  m_rejected(false) {
  initUi(input, initial);
  setWindowTitle("输入");
}

GvbSimInputDialog::~GvbSimInputDialog() {
  if (m_rejected) {
    for (const auto &field : m_input) {
      if (auto s = get_if<2>(&field)) {
        api::destroy_byte_string(*s);
      } else if (auto f = get_if<3>(&field)) {
        api::gvb_destroy_fn_body(*f);
      }
    }
  }
}

void GvbSimInputDialog::initUi(
  const api::GvbExecResult::KeyboardInput_Body &input,
  const api::Array<api::GvbKeyboardInput> *initial) {
  setStyleSheet(R"(
    QLabel#error {
      color: hsl(0, 100%, 50%);
    }
  )");

  auto layout = new QVBoxLayout(this);

  if (input.prompt.tag == api::Maybe<api::Utf8String>::Tag::Just) {
    layout->addWidget(new QLabel(QString("<b>%1</b>")
                                   .arg(QString::fromUtf8(
                                     input.prompt.just._0.data,
                                     input.prompt.just._0.len))));
  }

  auto form = new QFormLayout();
  form->setFieldGrowthPolicy(QFormLayout::AllNonFixedFieldsGrow);
  form->setLabelAlignment(Qt::AlignTop);
  layout->addLayout(form);

  QFont font("WenQuXing", 12);

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
        m_input[i] = InputField {in_place_index<0>, 0};
        auto input = new QSpinBox();
        fieldInput = input;
        input->setRange(-32768, 32767);
        input->setToolTip("范围：-32768 ~ 32767");
        if (initial) {
          input->setValue(initial->data[i].integer._0);
          input->focusWidget();
          input->selectAll();
        }
        connect(
          this,
          &GvbSimInputDialog::validateAll,
          input,
          &QSpinBox::editingFinished);
        connect(
          input,
          &QSpinBox::editingFinished,
          this,
          [i, input, this] {
            m_input[i] = static_cast<int16_t>(input->value());
            fieldValidated(true);
          },
          Qt::QueuedConnection);
        form->addRow("整数", input);
        break;
      }
      case api::GvbKeyboardInputType::Tag::Real: {
        m_input[i] = InputField {in_place_index<1>, api::GvbReal {0.0}};
        auto input = new DoubleSpinBox();
        fieldInput = input;
        input->setRange(-1.7e38, 1.7e38);
        input->setDecimals(6);
        input->setToolTip("范围：-1.7E+38 ~ +1.7E+38");
        if (initial) {
          input->setValue(initial->data[i].real._0._0);
          input->focusWidget();
          input->selectAll();
        }
        connect(
          this,
          &GvbSimInputDialog::validateAll,
          input,
          &QDoubleSpinBox::editingFinished);
        connect(
          input,
          &QSpinBox::editingFinished,
          this,
          [i, input, this] {
            m_input[i] = api::GvbReal {input->value()};
            fieldValidated(true);
          },
          Qt::QueuedConnection);
        form->addRow("实数", input);
        break;
      }
      case api::GvbKeyboardInputType::Tag::String: {
        m_input[i] = InputField {in_place_index<2>, ByteString {nullptr, 0}};
        auto layout = new QVBoxLayout();
        auto input = new QLineEdit();
        input->setFont(font);
        if (initial) {
          auto s = api::gvb_byte_string_to_utf8_lossy(
            m_vm,
            initial->data[i].string._0);
          input->setText(QString::fromUtf8(s.data, s.len));
          api::destroy_string(s);
          input->focusWidget();
          input->selectAll();
        }

        layout->addWidget(input);
        fieldInput = input;
        auto msg = new QLabel(" ");
        msg->setObjectName("error");
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
            auto result = api::gvb_utf16_to_byte_string(
              m_vm,
              {s.utf16(), static_cast<size_t>(s.size())});
            if (result.tag == api::GvbStringResult::Tag::Left) {
              switch (result.left._0.tag) {
                case api::GvbStringError::Tag::InvalidUtf16:
                  msg->setText("非法的 UTF-16 字符串");
                  break;
                case api::GvbStringError::Tag::InvalidChar: {
                  auto c = result.left._0.invalid_char._1;
                  msg->setText(QString("非法字符：%1 (U+%2)")
                                 // TODO check if c is printable
                                 .arg(QString::fromUcs4(&c, 1))
                                 .arg(
                                   static_cast<unsigned>(c),
                                   c <= 0xffff ? 4 : 6,
                                   16,
                                   QChar('0'))
                                 .toUpper());
                  break;
                }
              }
              return;
            }
            if (result.right._0.len > 255) {
              msg->setText(QString("字符串长度为 %1，超出上限 255")
                             .arg(result.right._0.len));
              fieldValidated(false);
            } else {
              msg->setText("");
              auto old = std::get<2>(m_input[i]);
              m_input[i] = InputField {in_place_index<2>, result.right._0};
              result.right._0 = old;
              fieldValidated(true);
            }
            api::destroy_byte_string(result.right._0);
          },
          Qt::QueuedConnection);
        form->addRow("字符串", layout);
        qobject_cast<QLabel *>(form->labelForField(layout))
          ->setAlignment(Qt::AlignLeft | Qt::AlignTop);
        break;
      }
      case api::GvbKeyboardInputType::Tag::Func: {
        m_input[i] = InputField {in_place_index<3>, nullptr};
        auto layout = new QVBoxLayout();
        auto input = new QLineEdit();
        input->setFont(font);
        layout->addWidget(input);
        fieldInput = input;
        auto msg = new QLabel(" ");
        msg->setObjectName("error");
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
            api::GvbDiagnostic<api::Utf8String> firstError;
            size_t firstErrorStart = std::numeric_limits<size_t>::max();
            for (auto p = result.diagnostics.data;
                 p < result.diagnostics.data + result.diagnostics.len;
                 p++) {
              if (
                p->severity == api::GvbSeverity::Error
                && (!error || p->start < firstErrorStart)) {
                error = true;
                firstError = *p;
                firstErrorStart = p->start;
              }
            }
            if (error) {
              msg->setText(QString("错误(第 %1 列)：%2")
                             .arg(firstError.start + 1)
                             .arg(QString::fromUtf8(
                               firstError.message.data,
                               firstError.message.len)));
              fieldValidated(false);
            } else {
              msg->setText("");
              auto old = std::get<3>(m_input[i]);
              m_input[i] = InputField {in_place_index<3>, result.body};
              result.body = old;
              fieldValidated(true);
            }
            api::gvb_destroy_fn_body(result.body);
            api::gvb_destroy_string_diagnostic_array(result.diagnostics);
          },
          Qt::QueuedConnection);
        form->addRow(
          QString("函数 %1(%2) =")
            .arg(QString::fromUtf8(field.func.name.data, field.func.name.len))
            .arg(
              QString::fromUtf8(field.func.param.data, field.func.param.len)),
          layout);
        qobject_cast<QLabel *>(form->labelForField(layout))
          ->setAlignment(Qt::AlignLeft | Qt::AlignTop);
        break;
      }
      default:
        qFatal("invalid keyboard input type: %d", static_cast<int>(field.tag));
    }

    if (lastField) {
      QWidget::setTabOrder(lastField, fieldInput);
    }
    lastField = fieldInput;
  }

  auto confirmLayout = new QHBoxLayout();
  layout->addLayout(confirmLayout);

  auto help = new QFrame();
  auto helpLayout = new QVBoxLayout();
  helpLayout->addWidget(new QLabel("?"));
  helpLayout->setContentsMargins(6, 0, 6, 0);
  help->setLayout(helpLayout);
  help->setFrameStyle(QFrame::Box);

#define COMMON_HELP "<b>Esc</b> 取消输入<br>"

  if (m_input.size() == 1) {
    help->setToolTip(COMMON_HELP "<b>Ctrl+Enter</b> 或 <b>Enter</b> 输入完毕");
  } else {
    help->setToolTip(COMMON_HELP "<b>Ctrl+Enter</b> 输入完毕");
  }
  confirmLayout->addWidget(help);

  auto confirm =
    new QDialogButtonBox(QDialogButtonBox::Ok | QDialogButtonBox::Cancel, this);
  confirmLayout->addWidget(confirm, 1);
  confirm->button(QDialogButtonBox::Ok)->setShortcut(Qt::CTRL | Qt::Key_Return);
  connect(confirm, &QDialogButtonBox::rejected, this, &QDialog::reject);
  connect(
    confirm,
    &QDialogButtonBox::accepted,
    this,
    &GvbSimInputDialog::startValidateAll);
}

QVector<api::GvbKeyboardInput> GvbSimInputDialog::inputData() {
  QVector<api::GvbKeyboardInput> result;
  for (const auto &field : m_input) {
    api::GvbKeyboardInput i;
    if (auto n = get_if<int16_t>(&field)) {
      i.tag = api::GvbKeyboardInput::Tag::Integer;
      i.integer._0 = *n;
      result.push_back(i);
    } else if (auto n = get_if<api::GvbReal>(&field)) {
      i.tag = api::GvbKeyboardInput::Tag::Real;
      i.real._0 = *n;
      result.push_back(i);
    } else if (auto s = get_if<2>(&field)) {
      i.tag = api::GvbKeyboardInput::Tag::String;
      i.string._0 = *s;
      result.push_back(i);
    } else if (auto f = get_if<3>(&field)) {
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
    accept();
  }
  m_validateAll = false;
  m_validatedFields = 0;
  m_validateOkFields = 0;
}

void GvbSimInputDialog::keyPressEvent(QKeyEvent *ev) {
  if (ev->key() == Qt::Key_Enter || ev->key() == Qt::Key_Return) {
    startValidateAll();
    return;
  }
  QDialog::keyPressEvent(ev);
}

void GvbSimInputDialog::reject() {
  m_rejected = true;
  QDialog::reject();
}