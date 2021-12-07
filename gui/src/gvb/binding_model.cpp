#include "binding_model.h"

#include <QDoubleSpinBox>
#include <QFont>
#include <QSpinBox>
#include <QTextStream>
#include <cstdint>
#include <stdexcept>

#include "gvbsim_input_dialog.h"

BindingModel::BindingModel(QWidget *parent) :
  m_vm(nullptr),
  m_bindings {nullptr, 0},
  m_enabled(false),
  m_parent(parent) {}

BindingModel::~BindingModel() {
  api::gvb_destroy_bindings(&m_bindings);
}

void BindingModel::setVm(api::GvbVirtualMachine *vm) {
  m_vm = vm;
  enable();
}

void BindingModel::disable() {
  m_enabled = false;
  beginResetModel();
  api::gvb_destroy_bindings(&m_bindings);
  endResetModel();
}

void BindingModel::enable() {
  m_enabled = true;

  if (!m_vm) {
    return;
  }

  auto oldLen = static_cast<int>(m_bindings.len);
  api::gvb_destroy_bindings(&m_bindings);
  m_bindings = api::gvb_vm_bindings(m_vm);
  auto newLen = static_cast<int>(m_bindings.len);
  if (newLen > oldLen) {
    beginInsertRows(QModelIndex(), oldLen, newLen - 1);
    endInsertRows();
  } else if (newLen < oldLen) {
    beginRemoveRows(QModelIndex(), newLen, oldLen - 1);
    endRemoveRows();
  }
  emit dataChanged(index(0, 0), index(newLen - 1, 1));
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
            QString result;
            QTextStream arr(&result);
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
              arr << *sub;
            }
            arr << ')';
            return result;
          }
        }
        break;
      }
      case Qt::TextAlignmentRole: {
        return static_cast<int>(Qt::AlignLeft | Qt::AlignVCenter);
      }
    }
  } else if (index.column() == 1) {
    if (!m_vm) {
      return QVariant();
    }
    switch (role) {
      case Qt::ToolTipRole:
        if (!m_enabled) {
          break;
        }
        // fall through
      case Qt::EditRole:
      case Qt::DisplayRole: {
        const auto &binding = m_bindings.data[index.row()];
        switch (binding.tag) {
          case api::GvbBinding::Tag::Var: {
            auto value = api::gvb_vm_var_value(
              m_vm,
              {binding.var.name.data, binding.var.name.len});
            switch (value.tag) {
              case api::GvbValue::Tag::Integer:
                api::gvb_destroy_value(value);
                return value.integer._0;
              case api::GvbValue::Tag::Real:
                api::gvb_destroy_value(value);
                return value.real._0._0;
              case api::GvbValue::Tag::String: {
                if (role == Qt::EditRole) {
                  api::gvb_destroy_value(value);
                  break;
                }
                auto s =
                  api::gvb_byte_string_to_utf8_lossy(m_vm, value.string._0);
                auto result = QString::fromUtf8(s.data, s.len);
                api::destroy_string(s);
                api::gvb_destroy_value(value);
                return result;
              }
            }
            break;
          }
          case api::GvbBinding::Tag::Array:
            if (role == Qt::EditRole) {
              break;
            }
            if (role == Qt::ToolTipRole) {
              return QString("双击修改数组");
            }
            return QString("<数组>");
        }
        break;
      }
      case Qt::FontRole: {
        const auto &binding = m_bindings.data[index.row()];
        if (
          binding.tag == api::GvbBinding::Tag::Var
          && binding.var.name.data[binding.var.name.len - 1] == '$') {
          return QFont("WenQuXing", 12);
        }
        break;
      }
      case Qt::TextAlignmentRole:
        return Qt::AlignCenter;
    }
  }

  return QVariant();
}

QWidget *
BindingModel::createEditor(QWidget *parent, const QModelIndex &index) const {
  if (index.column() != 1) {
    return nullptr;
  }

  const auto &binding = m_bindings.data[index.row()];
  switch (binding.tag) {
    case api::GvbBinding::Tag::Var: {
      switch (api::gvb_binding_type(&binding)) {
        case api::GvbBindingType::Integer: {
          auto box = new QSpinBox(parent);
          box->setRange(-32768, 32767);
          box->setToolTip("范围：-32768 ~ 32767");
          return box;
        }
        case api::GvbBindingType::Real: {
          auto box = new QDoubleSpinBox(parent);
          box->setRange(-1.7e38, 1.7e38);
          box->setDecimals(6);
          box->setToolTip("范围：-1.7E+38 ~ +1.7E+38");
          return box;
        }
        case api::GvbBindingType::String:
          throw std::logic_error("createEditor: string");
      }
      return nullptr;
    }
    case api::GvbBinding::Tag::Array:
      return nullptr;
    default:
      throw std::logic_error("createEditor: not var or array");
  }
}

void BindingModel::setEditorData(QWidget *editor, const QModelIndex &index)
  const {
  if (index.column() != 1) {
    return;
  }

  const auto &binding = m_bindings.data[index.row()];
  switch (binding.tag) {
    case api::GvbBinding::Tag::Var: {
      auto value = api::gvb_vm_var_value(
        m_vm,
        {binding.var.name.data, binding.var.name.len});
      switch (value.tag) {
        case api::GvbValue::Tag::Integer: {
          qobject_cast<QSpinBox *>(editor)->setValue(value.integer._0);
          break;
        }
        case api::GvbValue::Tag::Real: {
          qobject_cast<QDoubleSpinBox *>(editor)->setValue(value.real._0._0);
          break;
        }
        case api::GvbValue::Tag::String: {
          throw std::logic_error("setEditorData: string");
        }
      }
      api::gvb_destroy_value(value);
    }
    case api::GvbBinding::Tag::Array:
      return;
  }
}

void BindingModel::setData(QWidget *editor, const QModelIndex &index) {
  if (index.column() != 1) {
    return;
  }

  const auto &binding = m_bindings.data[index.row()];
  switch (binding.tag) {
    case api::GvbBinding::Tag::Var: {
      api::Utf8Str name {binding.var.name.data, binding.var.name.len};
      auto value = api::gvb_vm_var_value(m_vm, name);
      switch (value.tag) {
        case api::GvbValue::Tag::Integer: {
          auto n = static_cast<std::int16_t>(
            qobject_cast<QSpinBox *>(editor)->value());
          value.integer._0 = n;
          api::gvb_vm_modify_var(m_vm, name, value);
          break;
        }
        case api::GvbValue::Tag::Real: {
          auto n = qobject_cast<QDoubleSpinBox *>(editor)->value();
          value.real._0._0 = n;
          api::gvb_vm_modify_var(m_vm, name, value);
          break;
        }
        case api::GvbValue::Tag::String: {
          throw std::logic_error("setData: string");
        }
      }
      api::gvb_destroy_value(value);
      emit dataChanged(index, index);
      break;
    }
    case api::GvbBinding::Tag::Array:
      return;
  }
}

QVariant BindingModel::headerData(
  int section,
  Qt::Orientation orientation,
  int role) const {
  if (orientation == Qt::Horizontal) {
    switch (role) {
      case Qt::DisplayRole:
        switch (section) {
          case 0:
            return QString("变量名");
          case 1:
            return QString("值");
        }
        break;
      case Qt::ToolTipRole:
        if (!m_enabled) {
          break;
        }
        switch (section) {
          case 1:
            return QString("双击单元格修改变量值");
        }
        break;
    }
  }
  return QVariant();
}

Qt::ItemFlags BindingModel::flags(const QModelIndex &index) const {
  auto f = QAbstractTableModel::flags(index);
  if (index.column() == 1) {
    const auto &binding = m_bindings.data[index.row()];
    if (binding.tag == api::GvbBinding::Tag::Var) {
      switch (api::gvb_binding_type(&binding)) {
        case api::GvbBindingType::Integer:
        case api::GvbBindingType::Real:
          return Qt::ItemIsEditable | f;
        case api::GvbBindingType::String:
          return f;
      }
    }
  }
  return f;
}

void BindingModel::editValue(const QModelIndex &index) {
  if (index.column() != 1) {
    return;
  }

  const auto &binding = m_bindings.data[index.row()];
  if (binding.tag == api::GvbBinding::Tag::Var) {
    if (api::gvb_binding_type(&binding) != api::GvbBindingType::String) {
      return;
    }

    api::Utf8Str name = {binding.var.name.data, binding.var.name.len};

    // edit string
    api::GvbExecResult res;
    res.tag = api::GvbExecResult::Tag::KeyboardInput;
    res.keyboard_input.prompt.tag = api::Maybe<api::Utf8String>::Tag::Nothing;
    api::GvbKeyboardInputType types[1];
    types[0].tag = api::GvbKeyboardInputType::Tag::String;
    res.keyboard_input.fields.data = types;
    res.keyboard_input.fields.len = 1;

    auto value = api::gvb_vm_var_value(m_vm, name);

    api::GvbKeyboardInput initial[1];
    initial[0].tag = api::GvbKeyboardInput::Tag::String;
    initial[0].string._0 = value.string._0;
    auto initialInput = api::gvb_new_input_array(initial, 1);
    //api::gvb_destroy_value(value);
    GvbSimInputDialog dlg(m_parent, m_vm, res.keyboard_input, &initialInput);
    api::gvb_destroy_input_array(initialInput);
    dlg.setWindowTitle(tr("修改变量 %1").arg(
      QString::fromUtf8(name.data, static_cast<int>(name.len))));
    dlg.setModal(true);
    if (dlg.exec() == QDialog::Rejected) {
      return;
    }

    value.tag = api::GvbValue::Tag::String;
    value.string._0 = dlg.inputData()[0].string._0;

    api::gvb_vm_modify_var(m_vm, name, value);
    emit dataChanged(index, index);

    return;
  }

  // TODO edit array
}