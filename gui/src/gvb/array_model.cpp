#include "array_model.h"

#include <QDoubleSpinBox>
#include <QFont>
#include <QSpinBox>
#include <stdexcept>

#include "gvb_util.h"

ArrayModel::ArrayModel(
  QWidget *parent,
  api::GvbVirtualMachine *vm,
  const api::GvbBinding::Array_Body &array) :
  m_parent(parent),
  m_vm(vm),
  m_name({array.name.data, array.name.len}),
  m_bounds(array.dimensions),
  m_subscripts(array.dimensions.len),
  m_rowDim(0),
  m_colDim(0) {}

ArrayModel::~ArrayModel() {
  destroyData();
}

int ArrayModel::rowCount(const QModelIndex &) const {
  return m_bounds.data[m_rowDim];
}

int ArrayModel::columnCount(const QModelIndex &) const {
  if (m_bounds.len == 1) {
    return 1;
  } else {
    return m_bounds.data[m_colDim];
  }
}

QVariant ArrayModel::data(const QModelIndex &index, int role) const {
  switch (role) {
    case Qt::ToolTipRole:
    case Qt::DisplayRole:
      if (auto iarr = std::get_if<0>(&m_data)) {
        return (*iarr)[index.column()].data[index.row()];
      } else if (auto farr = std::get_if<1>(&m_data)) {
        return (*farr)[index.column()].data[index.row()]._0;
      } else if (auto sarr = std::get_if<2>(&m_data)) {
        const auto &bs = (*sarr)[index.column()].data[index.row()];
        auto us = api::gvb_byte_string_to_utf8_lossy(m_vm, bs);
        auto s = QString::fromUtf8(us.data, us.len);
        api::destroy_string(us);
        return s;
      }
      break;
    case Qt::FontRole:
      if (m_data.index() == 2) {
        return QFont("WenQuXing", 12);
      }
      break;
    case Qt::TextAlignmentRole:
      return Qt::AlignCenter;
  }
  return QVariant();
}

QWidget *ArrayModel::createEditor(QWidget *parent, const QModelIndex &) const {
  switch (m_data.index()) {
    case 0: {
      auto box = new QSpinBox(parent);
      box->setRange(-32768, 32767);
      box->setToolTip("范围：-32768 ~ 32767");
      return box;
    }
    case 1: {
      auto box = new QDoubleSpinBox(parent);
      box->setRange(-1.7e38, 1.7e38);
      box->setDecimals(6);
      box->setToolTip("范围：-1.7E+38 ~ +1.7E+38");
      return box;
    }
    case 2: {
      throw std::logic_error("createEditor: string");
    }
  }
  return nullptr;
}

void ArrayModel::setEditorData(QWidget *editor, const QModelIndex &index)
  const {
  if (auto iarr = std::get_if<0>(&m_data)) {
    qobject_cast<QSpinBox *>(editor)->setValue(
      (*iarr)[index.column()].data[index.row()]);
  } else if (auto farr = std::get_if<1>(&m_data)) {
    qobject_cast<QDoubleSpinBox *>(editor)->setValue(
      (*farr)[index.column()].data[index.row()]._0);
  } else if (std::get_if<2>(&m_data)) {
    throw std::logic_error("setEditorData: string");
  }
}

QVariant ArrayModel::headerData(
  int section,
  Qt::Orientation orientation,
  int role) const {
  if (orientation == Qt::Horizontal && m_bounds.len == 1) {
    return QVariant();
  }
  switch (role) {
    case Qt::DisplayRole:
      return section;
    case Qt::ToolTipRole:
      return QString("双击单元格修改数组元素");
  }
  return QVariant();
}

Qt::ItemFlags ArrayModel::flags(const QModelIndex &index) const {
  auto f = QAbstractTableModel::flags(index);
  if (m_data.index() == 2) {
    return f;
  }
  return Qt::ItemIsEditable | f;
}

void ArrayModel::setData(QWidget *editor, const QModelIndex &index) {
  api::GvbValue value;
  const auto subVec = calcSubs(index);
  const api::Array<std::uint16_t> subs {
    subVec.data(),
    static_cast<size_t>(subVec.size())};
  if (auto iarr = std::get_if<0>(&m_data)) {
    auto n =
      static_cast<std::int16_t>(qobject_cast<QSpinBox *>(editor)->value());
    value.tag = api::GvbValue::Tag::Integer;
    value.integer._0 = n;
    (*iarr)[index.column()].data[index.row()] = n;
    api::gvb_vm_modify_arr(m_vm, m_name, subs, value);
  } else if (auto farr = std::get_if<1>(&m_data)) {
    auto n = qobject_cast<QDoubleSpinBox *>(editor)->value();
    value.tag = api::GvbValue::Tag::Real;
    value.real._0._0 = n;
    (*farr)[index.column()].data[index.row()]._0 = n;
    api::gvb_vm_modify_arr(m_vm, m_name, subs, value);
  } else {
    throw std::logic_error("setData: string");
  }
  api::gvb_destroy_value(value);
  emit dataChanged(index, index, {Qt::DisplayRole, Qt::ToolTipRole});
}

QVector<std::uint16_t> ArrayModel::calcSubs(const QModelIndex &index) const {
  QVector subs(m_subscripts);
  subs[m_colDim] = index.column();
  subs[m_rowDim] = index.row();
  return subs;
}

void ArrayModel::editValue(const QModelIndex &index) {
  auto sarr = std::get_if<2>(&m_data);
  if (!sarr) {
    return;
  }

  // edit string
  auto subVec = calcSubs(index);
  const api::Array<std::uint16_t> subs {
    subVec.data(),
    static_cast<size_t>(subVec.size())};
  auto result = inputString(
    m_parent,
    m_vm,
    tr("修改数组元素 %1").arg(arraySubsToString(m_name, subs)),
    api::copy_byte_string((*sarr)[index.column()].data[index.row()]));
  if (result.has_value()) {
    auto value = result.value();
    auto &s = (*sarr)[index.column()].data[index.row()];
    api::destroy_byte_string(s);
    s = api::copy_byte_string(value.string._0);
    api::gvb_vm_modify_var(m_vm, m_name, value);
    emit dataChanged(index, index, {Qt::DisplayRole, Qt::ToolTipRole});
  }
}

void ArrayModel::setSubscript(size_t index, std::uint16_t sub) {
  m_subscripts[index] = sub;
  loadData(m_rowDim, m_colDim);
}

void ArrayModel::setPlaneDim(size_t rowDim, size_t colDim) {
  loadData(rowDim, colDim);
}

void ArrayModel::loadData(size_t newRowDim, size_t newColDim) {
  destroyData();
  api::GvbDimensionValues::Tag type = ;
}

void ArrayModel::destroyData() {
  if (auto iarr = std::get_if<0>(&m_data)) {
    for (const auto &arr : *iarr) {
      api::destroy_i16_array_mut(arr);
    }
    iarr->clear();
  } else if (auto farr = std::get_if<1>(&m_data)) {
    for (const auto &arr : *farr) {
      api::gvb_destroy_real_array_mut(arr);
    }
    farr->clear();
  } else if (auto sarr = std::get_if<2>(&m_data)) {
    for (const auto &arr : *sarr) {
      api::destroy_byte_string_array_mut(arr);
    }
    sarr->clear();
  }
}