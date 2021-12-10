#pragma once

#include <QDialog>
#include <QVector>
#include <cstdint>

#include "api.h"
#include "array_model.h"
#include "table_editor_delegate.h"

class QGridLayout;
class QSpinBox;
class QButtonGroup;

class ArrayEditDialog: public QDialog {
  Q_OBJECT

public:
  ArrayEditDialog(
    QWidget *parent,
    const api::GvbBinding::Array_Body &array,
    api::GvbVirtualMachine *vm);
  ~ArrayEditDialog();

private:
  void initUi(const api::GvbBinding::Array_Body &);
  QGridLayout *initDimensionSelector(const api::GvbBinding::Array_Body &);

private slots:
  void setRowDim(int);
  void setColDim(int);

private:
  const api::Array<std::uint16_t> m_bounds;
  const api::GvbVirtualMachine *m_vm;
  ArrayModel m_arrayModel;
  TableEditorDelegate m_arrayDelegate;
  QVector<QSpinBox *> m_spinBoxes;
  QButtonGroup *m_rowGroup;
  QButtonGroup *m_colGroup;
  size_t m_curRowDim;
  size_t m_curColDim;
};