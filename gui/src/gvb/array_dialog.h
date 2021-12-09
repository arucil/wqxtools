#pragma once

#include <QDialog>
#include <QVector>
#include <cstdint>
#include <optional>

#include "api.h"
#include "array_model.h"
#include "table_editor_delegate.h"

using std::optional;

class QGridLayout;
class QCheckBox;
class QSpinBox;

struct DimensionSelectorItem {
  QSpinBox *sub;
  QCheckBox *row;
  QCheckBox *col;
};

class ArrayDialog: public QDialog {
  Q_OBJECT

public:
  ArrayDialog(
    QWidget *parent,
    const api::GvbBinding::Array_Body &array,
    api::GvbVirtualMachine *vm);
  ~ArrayDialog();

private:
  void initUi(const api::GvbBinding::Array_Body &);
  QGridLayout *initDimensionSelector(const api::GvbBinding::Array_Body &);
  void setRowDim(const optional<size_t> &);
  void setColDim(const optional<size_t> &);

private:
  const api::Array<std::uint16_t> m_bounds;
  const api::GvbVirtualMachine *m_vm;
  ArrayModel m_arrayModel;
  TableEditorDelegate m_arrayDelegate;
  QVector<DimensionSelectorItem> m_dimSelItems;
  optional<size_t> m_curRowDim;
  optional<size_t> m_curColDim;
};