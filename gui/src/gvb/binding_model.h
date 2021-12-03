#pragma once

#include <QAbstractTableModel>

#include "api.h"

class BindingModel: public QAbstractTableModel {
  Q_OBJECT

public:
  BindingModel(QWidget *parent);
  ~BindingModel();

  void setVm(api::GvbVirtualMachine *);
  void enable();
  void disable();

  QWidget *createEditor(QWidget *parent, const QModelIndex &index) const;
  void setEditorData(QWidget *editor, const QModelIndex &) const;
  void setData(QWidget *editor, const QModelIndex &);

public:
  int rowCount(const QModelIndex &parent) const override;
  int columnCount(const QModelIndex &parent) const override;
  QVariant data(const QModelIndex &index, int role) const override;
  QVariant
  headerData(int section, Qt::Orientation orientation, int role) const override;
  Qt::ItemFlags flags(const QModelIndex &) const override;

public slots:
  void editValue(const QModelIndex &);

private:
  api::GvbVirtualMachine *m_vm;
  api::Array<api::GvbBinding> m_bindings;
  bool m_enabled;
  QWidget *m_parent;
};