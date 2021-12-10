#pragma once

#include <QAbstractTableModel>

#include "api.h"
#include "table_editor_model.h"

class BindingModel: public TableEditorModel {
  Q_OBJECT

public:
  BindingModel(QWidget *parent);
  ~BindingModel();

  void setVm(api::GvbVirtualMachine *);
  void enable();
  void disable();

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
  api::GvbVirtualMachine *m_vm;
  api::ArrayMut<api::GvbBinding> m_bindings;
  bool m_enabled;
  QWidget *m_parent;
};