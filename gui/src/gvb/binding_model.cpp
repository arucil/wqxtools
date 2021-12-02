#include "binding_model.h"

#include <QTextStream>

BindingModel::BindingModel() : m_vm(nullptr) {}

BindingModel::~BindingModel() {
  api::gvb_destroy_bindings(m_bindings);
}

void BindingModel::setVm(api::GvbVirtualMachine *vm) {
  m_vm = vm;
}

void BindingModel::disable() {
  api::gvb_destroy_bindings(m_bindings);
  m_bindings.data = nullptr;
  m_bindings.len = 0;
}

void BindingModel::enable() {
  if (m_vm) {
    m_bindings = api::gvb_vm_bindings(m_vm);
    printf(">>>>>>>>. %lu\n", m_bindings.len);
  }
}

int BindingModel::columnCount(const QModelIndex &) const {
  return 2;
}

int BindingModel::rowCount(const QModelIndex &) const {
  return static_cast<int>(m_bindings.len);
}

QVariant BindingModel::data(const QModelIndex &index, int role) const {
  if (index.column() == 0) {
    switch (role) {
      case Qt::DisplayRole: {
        const auto &binding = m_bindings.data[index.row()];
        switch (binding.tag) {
          case api::GvbBinding::Tag::Var:
            return QString::fromUtf8(
              binding.var.name.data,
              binding.var.name.len);
          case api::GvbBinding::Tag::Array: {
            QTextStream arr;
            arr << QString::fromUtf8(
              binding.array.name.data,
              binding.array.name.len);
            arr << '(';
            auto dimensions = binding.array.dimensions;
            auto comma = false;
            for (auto sub = dimensions.data;
                 sub < dimensions.data + dimensions.len;
                 sub++) {
              if (comma) {
                arr << ",";
              }
              comma = true;
              arr << sub;
            }
            arr << ')';
            return *arr.string();
          }
        }
        break;
      }
      case Qt::TextAlignmentRole: {
        return Qt::AlignLeft;
      }
    }
  } else if (index.column() == 1) {
    if (!m_vm) {
      return QVariant();
    }
    switch (role) {
      case Qt::DisplayRole: {
        const auto &binding = m_bindings.data[index.row()];
        switch (binding.tag) {
          case api::GvbBinding::Tag::Var: {
            auto value = api::gvb_vm_var_value(
              m_vm,
              {binding.var.name.data, binding.var.name.len});
            switch (value.tag) {
              case api::GvbValue::Tag::Integer:
                return QString::number(value.integer._0);
              case api::GvbValue::Tag::Real:
                return QString::number(value.real._0._0);
              case api::GvbValue::Tag::String: {
                auto s =
                  api::gvb_byte_string_to_utf8_lossy(m_vm, value.string._0);
                auto result = QString::fromUtf8(s.data, s.len);
                api::destroy_string(s);
                return result;
              }
            }
            break;
          }
          case api::GvbBinding::Tag::Array:
            return QString("...");
        }
        break;
      }
      case Qt::TextAlignmentRole:
        return Qt::AlignCenter;
    }
  }

  return QVariant();
}

QVariant BindingModel::headerData(
  int section,
  Qt::Orientation orientation,
  int role) const {
  if (role == Qt::DisplayRole && orientation == Qt::Horizontal) {
    switch (section) {
      case 0:
        return QString("变量名");
      case 1:
        return QString("数据");
    }
  }
  return QVariant();
}