#include "array_model.h"

#include <QDoubleSpinBox>
#include <QFont>
#include <QSpinBox>

#include "gvb_util.h"

using std::uint16_t;
using std::int16_t;
using std::get_if;

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
  m_colDim(0),
  m_rows(0),
  m_cols(0) {}

ArrayModel::~ArrayModel() {
  destroyData();
}

int ArrayModel::rowCount(const QModelIndex &) const {
  return m_rows;
}

int ArrayModel::columnCount(const QModelIndex &) const {
  return m_cols;
}

QVariant ArrayModel::data(const QModelIndex &index, int role) const {
  switch (role) {
    case Qt::ToolTipRole:
    case Qt::DisplayRole:
      if (auto iarr = get_if<0>(&m_data)) {
        return (*iarr)[index.row()].data[index.column()];
      } else if (auto farr = get_if<1>(&m_data)) {
        return (*farr)[index.row()].data[index.column()]._0;
      } else if (auto sarr = get_if<2>(&m_data)) {
        const auto &bs = (*sarr)[index.row()].data[index.column()];
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
      qFatal("createEditor: string");
    }
  }
  return nullptr;
}

void ArrayModel::setEditorData(QWidget *editor, const QModelIndex &index)
  const {
  if (auto iarr = get_if<0>(&m_data)) {
    qobject_cast<QSpinBox *>(editor)->setValue(
      (*iarr)[index.row()].data[index.column()]);
  } else if (auto farr = get_if<1>(&m_data)) {
    qobject_cast<QDoubleSpinBox *>(editor)->setValue(
      (*farr)[index.row()].data[index.column()]._0);
  } else if (get_if<2>(&m_data)) {
    qFatal("setEditorData: string");
  }
}

QVariant ArrayModel::headerData(
  int section,
  Qt::Orientation orientation,
  int role) const {
  if (orientation == Qt::Vertical && m_bounds.len == 1) {
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
  const auto subVec = getSubs(index);
  const api::Array<uint16_t> subs {
    subVec.data(),
    static_cast<size_t>(subVec.size())};
  if (auto iarr = get_if<0>(&m_data)) {
    auto n =
      static_cast<int16_t>(qobject_cast<QSpinBox *>(editor)->value());
    value.tag = api::GvbValue::Tag::Integer;
    value.integer._0 = n;
    (*iarr)[index.row()].data[index.column()] = n;
    api::gvb_vm_modify_arr(m_vm, m_name, subs, value);
  } else if (auto farr = get_if<1>(&m_data)) {
    auto n = qobject_cast<QDoubleSpinBox *>(editor)->value();
    value.tag = api::GvbValue::Tag::Real;
    value.real._0._0 = n;
    (*farr)[index.row()].data[index.column()]._0 = n;
    api::gvb_vm_modify_arr(m_vm, m_name, subs, value);
  } else {
    qFatal("setData: string");
  }
  api::gvb_destroy_value(value);
  emit dataChanged(index, index, {Qt::DisplayRole, Qt::ToolTipRole});
}

QVector<uint16_t> ArrayModel::getSubs(const QModelIndex &index) const {
  QVector subs(m_subscripts);
  subs[m_colDim] = index.column();
  if (m_bounds.len > 1) {
    subs[m_rowDim] = index.row();
  }
  return subs;
}

void ArrayModel::editValue(const QModelIndex &index) {
  auto sarr = get_if<2>(&m_data);
  if (!sarr) {
    return;
  }

  // edit string
  auto subVec = getSubs(index);
  const api::Array<uint16_t> subs {
    subVec.data(),
    static_cast<size_t>(subVec.size())};
  auto result = inputString(
    m_parent,
    m_vm,
    QString("修改数组元素 %1").arg(arraySubsToString(m_name, subs)),
    api::copy_byte_string((*sarr)[index.row()].data[index.column()]));
  if (result.has_value()) {
    auto value = result.value();
    auto &s = (*sarr)[index.row()].data[index.column()];
    api::destroy_byte_string(s);
    s = api::copy_byte_string(value.string._0);
    api::gvb_vm_modify_arr(m_vm, m_name, subs, value);
    emit dataChanged(index, index, {Qt::DisplayRole, Qt::ToolTipRole});
  }
}

void ArrayModel::setSubscript(size_t index, uint16_t sub) {
  m_subscripts[index] = sub;
  loadData(m_rowDim, m_colDim);
}

void ArrayModel::setPlaneDim(size_t rowDim, size_t colDim) {
  if (m_bounds.len == 1 || rowDim != m_rowDim || colDim != m_colDim) {
    loadData(rowDim, colDim);
  }
}

void ArrayModel::loadData(size_t newRowDim, size_t newColDim) {
  destroyData();
  auto subVec = m_subscripts;
  auto fontChanged = false;
  uint16_t bound = m_bounds.len == 1 ? 0 : m_bounds.data[newRowDim];
  for (uint16_t i = 0; i <= bound; i++) {
    subVec[newRowDim] = i;
    auto values = api::gvb_vm_arr_dim_values(
      m_vm,
      m_name,
      {subVec.constData(), static_cast<size_t>(subVec.size())},
      newColDim);
    switch (values.tag) {
      case api::GvbDimensionValues::Tag::Integer: {
        if (auto iarr = get_if<0>(&m_data)) {
          iarr->push_back(values.integer._0);
        } else {
          m_data = QVector {values.integer._0};
        }
        break;
      }
      case api::GvbDimensionValues::Tag::Real: {
        if (auto iarr = get_if<1>(&m_data)) {
          iarr->push_back(values.real._0);
        } else {
          m_data = QVector {values.real._0};
        }
        break;
      }
      case api::GvbDimensionValues::Tag::String: {
        if (auto iarr = get_if<2>(&m_data)) {
          iarr->push_back(values.string._0);
        } else {
          fontChanged = true;
          m_data = QVector {values.string._0};
        }
        break;
      }
    }
  }

  auto oldRows = m_rows;
  auto newRows = m_bounds.data[newRowDim] + 1;
  if (m_bounds.len == 1) {
    if (m_rows != 1) {
      beginInsertRows(QModelIndex(), 0, 0);
      endInsertRows();
    }
    m_rows = 1;
  } else if (newRows != oldRows) {
    if (newRows > oldRows) {
      beginInsertRows(QModelIndex(), oldRows, newRows - 1);
      endInsertRows();
    } else if (newRows < oldRows) {
      beginRemoveRows(QModelIndex(), newRows, oldRows - 1);
      endRemoveRows();
    }
    m_rowDim = newRowDim;
    m_rows = newRows;
  }

  auto oldCols = m_cols;
  auto newCols = m_bounds.data[newColDim] + 1;
  if (newCols != oldCols) {
    if (newCols > oldCols) {
      beginInsertColumns(QModelIndex(), oldCols, newCols - 1);
      endInsertColumns();
    } else if (newCols < oldCols) {
      beginRemoveColumns(QModelIndex(), newCols, oldCols - 1);
      endRemoveColumns();
    }
    m_colDim = newColDim;
    m_cols = newCols;
  }

  QVector<int> changed {Qt::ToolTipRole, Qt::DisplayRole};
  if (fontChanged) {
    changed.push_back(Qt::FontRole);
  }
  emit dataChanged(index(0, 0), index(m_rows, m_cols), changed);
}

void ArrayModel::destroyData() {
  if (auto iarr = get_if<0>(&m_data)) {
    for (const auto &arr : *iarr) {
      api::destroy_i16_array_mut(arr);
    }
    iarr->clear();
  } else if (auto farr = get_if<1>(&m_data)) {
    for (const auto &arr : *farr) {
      api::gvb_destroy_real_array_mut(arr);
    }
    farr->clear();
  } else if (auto sarr = get_if<2>(&m_data)) {
    for (const auto &arr : *sarr) {
      api::destroy_byte_string_array_mut(arr);
    }
    sarr->clear();
  }
}