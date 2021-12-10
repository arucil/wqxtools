#include "array_edit_dialog.h"

#include <QButtonGroup>
#include <QDialogButtonBox>
#include <QGridLayout>
#include <QLabel>
#include <QRadioButton>
#include <QSpinBox>
#include <QTableView>
#include <QTimer>
#include <QVBoxLayout>

#include "gvb_util.h"

ArrayEditDialog::ArrayEditDialog(
  QWidget *parent,
  const api::GvbBinding::Array_Body &array,
  api::GvbVirtualMachine *vm) :
  QDialog(parent),
  m_bounds(array.dimensions),
  m_vm(vm),
  m_arrayModel(parent, vm, array),
  m_spinBoxes(array.dimensions.len),
  m_rowGroup(nullptr),
  m_colGroup(nullptr),
  m_curRowDim(0),
  m_curColDim(0) {
  initUi(array);
  adjustSize();
  setWindowTitle(tr("修改数组 %1").arg(arrayBindingName(array)));
  QTimer::singleShot(0, [this] {
    if (m_bounds.len > 1) {
      m_curRowDim = 1;
      m_curColDim = 0;
      m_arrayModel.setPlaneDim(1, 0);
    } else {
      m_arrayModel.setPlaneDim(0, 0);
    }
  });
}

ArrayEditDialog::~ArrayEditDialog() {
  delete m_colGroup;
  delete m_rowGroup;
}

void ArrayEditDialog::initUi(const api::GvbBinding::Array_Body &array) {
  auto layout = new QVBoxLayout(this);

  auto d = initDimensionSelector(array);
  if (d) {
    layout->addLayout(d);
  }

  auto arrayView = new QTableView();
  arrayView->setModel(&m_arrayModel);
  arrayView->setItemDelegate(&m_arrayDelegate);
  connect(
    arrayView,
    &QTableView::doubleClicked,
    &m_arrayModel,
    &ArrayModel::editValue);
  layout->addWidget(arrayView);

  layout->addWidget(
    new QDialogButtonBox(QDialogButtonBox::Ok),
    0,
    Qt::AlignRight);
}

QGridLayout *ArrayEditDialog::initDimensionSelector(
  const api::GvbBinding::Array_Body &array) {
  if (array.dimensions.len == 1) {
    return nullptr;
  }

  auto grid = new QGridLayout(this);
  grid->addWidget(new QLabel("下标"), 0, 0);
  grid->addWidget(new QLabel("行(Y轴)"), 1, 0);
  grid->addWidget(new QLabel("列(X轴)"), 2, 0);

  m_rowGroup = new QButtonGroup();
  connect(
    m_rowGroup,
    &QButtonGroup::idClicked,
    this,
    &ArrayEditDialog::setRowDim);
  m_colGroup = new QButtonGroup();
  connect(
    m_colGroup,
    &QButtonGroup::idClicked,
    this,
    &ArrayEditDialog::setColDim);

  for (size_t i = 0; i < array.dimensions.len; i++) {
    auto spin = new QSpinBox();
    m_spinBoxes[i] = spin;
    connect(
      spin,
      QOverload<int>::of(&QSpinBox::valueChanged),
      this,
      [i, this](int sub) { m_arrayModel.setSubscript(i, sub); });
    spin->setRange(0, array.dimensions.data[i]);
    if (i < 2) {
      spin->setEnabled(false);
    }
    grid->addWidget(spin, 0, i + 1);

    auto row = new QRadioButton();
    if (i == 1) {
      row->setChecked(true);
    }
    grid->addWidget(row, 1, i + 1);
    m_rowGroup->addButton(row, i);

    auto col = new QRadioButton();
    if (i == 0) {
      col->setChecked(true);
    }
    grid->addWidget(col, 2, i + 1);
    m_colGroup->addButton(col, i);
  }
  return grid;
}

void ArrayEditDialog::setRowDim(int i) {
  if (m_curColDim == static_cast<size_t>(i)) {
    m_curColDim = m_curRowDim;
    m_colGroup->button(m_curColDim)->click();
  } else {
    m_spinBoxes[m_curRowDim]->setEnabled(true);
    m_spinBoxes[i]->setEnabled(false);
  }
  m_curRowDim = i;
  m_arrayModel.setPlaneDim(m_curRowDim, m_curColDim);
}

void ArrayEditDialog::setColDim(int i) {
  if (m_curRowDim == static_cast<size_t>(i)) {
    m_curRowDim = m_curColDim;
    m_rowGroup->button(m_curRowDim)->click();
  } else {
    m_spinBoxes[m_curColDim]->setEnabled(true);
    m_spinBoxes[i]->setEnabled(false);
  }
  m_curColDim = i;
  m_arrayModel.setPlaneDim(m_curRowDim, m_curColDim);
}