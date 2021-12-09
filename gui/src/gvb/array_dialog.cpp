#include "array_dialog.h"

#include <QCheckBox>
#include <QDialogButtonBox>
#include <QGridLayout>
#include <QLabel>
#include <QSpinBox>
#include <QTableView>
#include <QTimer>
#include <QVBoxLayout>

#include "gvb_util.h"

ArrayDialog::ArrayDialog(
  QWidget *parent,
  const api::GvbBinding::Array_Body &array,
  api::GvbVirtualMachine *vm) :
  QDialog(parent),
  m_bounds(array.dimensions),
  m_vm(vm),
  m_arrayModel(parent, vm, array),
  m_dimSelItems(array.dimensions.len),
  m_curRowDim(-1),
  m_curColDim(-1) {
  initUi(array);
  adjustSize();
  setWindowTitle(tr("修改数组 %1").arg(array_binding_name(array)));
  QTimer::singleShot(0, [this] {
    if (m_bounds.len > 1) {
      setRowDim(0);
      setColDim(1);
    } else {
      m_arrayModel.setPlaneDim(0, -1);
    }
  });
}

ArrayDialog::~ArrayDialog() {}

void ArrayDialog::initUi(const api::GvbBinding::Array_Body &array) {
  auto layout = new QVBoxLayout(this);

  auto d = initDimensionSelector(array);
  if (d) {
    layout->addLayout(d);
  }

  auto arrayView = new QTableView();
  arrayView->setModel(&m_arrayModel);
  arrayView->setItemDelegate(&m_arrayDelegate);
  layout->addWidget(arrayView);

  layout->addWidget(
    new QDialogButtonBox(QDialogButtonBox::Ok),
    0,
    Qt::AlignRight);
}

QGridLayout *
ArrayDialog::initDimensionSelector(const api::GvbBinding::Array_Body &array) {
  if (array.dimensions.len == 1) {
    return nullptr;
  }

  auto grid = new QGridLayout(this);
  grid->addWidget(new QLabel("下标"), 0, 0);
  grid->addWidget(new QLabel("行(Y轴)"), 1, 0);
  grid->addWidget(new QLabel("列(X轴)"), 2, 0);
  for (int i = 0; i < array.dimensions.len; i++) {
    auto spin = new QSpinBox();
    connect(
      spin,
      QOverload<int>::of(&QSpinBox::valueChanged),
      this,
      [i, this](int sub) {
        m_arrayModel.setSubscript(i, sub);
      });
    spin->setRange(0, array.dimensions.data[i]);
    auto row = new QCheckBox();
    connect(row, &QCheckBox::stateChanged, this, [i, this](int checked) {
      if (checked == Qt::Checked) {
        setRowDim(i);
      } else {
        setRowDim({});
      }
    });
    auto col = new QCheckBox();
    connect(col, &QCheckBox::stateChanged, this, [i, this](int checked) {
      if (checked == Qt::Checked) {
        setColDim(i);
      } else {
        setColDim({});
      }
    });
    m_dimSelItems[i] = {spin, row, col};
  }
  return grid;
}

void ArrayDialog::setRowDim(const optional<size_t> &row) {
  if (!row.has_value()) {
    if (m_curRowDim.has_value()) {
      auto &sel = m_dimSelItems[m_curRowDim.value()];
      sel.sub->setEnabled(true);
      sel.
    }
    return;
  }
}