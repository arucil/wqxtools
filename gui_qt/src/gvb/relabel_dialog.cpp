#include "relabel_dialog.h"

#include <QDialogButtonBox>
#include <QFormLayout>
#include <QSpinBox>

RelabelDialog::RelabelDialog(QWidget *parent) : QDialog(parent) {
  initUi();
}

void RelabelDialog::initUi() {
  auto layout = new QFormLayout(this);
  auto start = new QSpinBox(this);
  start->setRange(0, 9999);
  start->setValue(10);
  layout->addRow("起始行号", start);
  auto inc = new QSpinBox(this);
  inc->setRange(1, 9999);
  inc->setValue(10);
  layout->addRow("行号步长", inc);

  auto btns =
    new QDialogButtonBox(QDialogButtonBox::Yes | QDialogButtonBox::No, this);
  layout->addRow(btns);

  connect(btns, &QDialogButtonBox::rejected, this, &QDialog::hide);
  connect(btns, &QDialogButtonBox::accepted, this, [=] {
    hide();
    emit relabel(
      static_cast<uint16_t>(start->value()),
      static_cast<uint16_t>(inc->value()));
  });

  setWindowTitle("重排行号");
}