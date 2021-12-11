#pragma once

#include <QAbstractTableModel>
#include <QVector>
#include <cstdint>
#include <variant>

#include "api.h"
#include "table_editor_model.h"

typedef std::variant<
  QVector<api::ArrayMut<std::int16_t>>,
  QVector<api::ArrayMut<api::GvbReal>>,
  QVector<api::ArrayMut<api::Array<std::uint8_t>>>>
  ArrayPlaneData;

class ArrayModel: public TableEditorModel {
  Q_OBJECT
public:
  ArrayModel(
    QWidget *parent,
    api::GvbVirtualMachine *,
    const api::GvbBinding::Array_Body &);
  ~ArrayModel();

  void setSubscript(size_t index, std::uint16_t sub);
  void setPlaneDim(size_t rowDim, size_t colDim);

public:
  int rowCount(const QModelIndex &parent) const override;
  int columnCount(const QModelIndex &parent) const override;
  QVariant data(const QModelIndex &index, int role) const override;
  QVariant
  headerData(int section, Qt::Orientation orientation, int role) const override;
  Qt::ItemFlags flags(const QModelIndex &) const override;

  QWidget *
  createEditor(QWidget *parent, const QModelIndex &index) const override;
  void setEditorData(QWidget *editor, const QModelIndex &) const override;
  void setData(QWidget *editor, const QModelIndex &) override;

public slots:
  void editValue(const QModelIndex &);

private:
  void loadData(size_t newRowDim, size_t newColDim);
  QVector<std::uint16_t> getSubs(const QModelIndex &) const;
  void destroyData();

private:
  QWidget *m_parent;
  api::GvbVirtualMachine *m_vm;
  ArrayPlaneData m_data;
  api::Utf8Str m_name;
  api::Array<std::uint16_t> m_bounds;
  QVector<std::uint16_t> m_subscripts;
  size_t m_rowDim, m_colDim;
  std::uint16_t m_rows, m_cols;
};