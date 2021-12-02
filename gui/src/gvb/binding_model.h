#pragma once

#include <QAbstractTableModel>

#include "api.h"

class BindingModel: public QAbstractTableModel {
  Q_OBJECT

public:
  BindingModel();
  ~BindingModel();

  void setVm(api::GvbVirtualMachine *);

public:
  int rowCount(const QModelIndex &parent) const Q_DECL_OVERRIDE;
  int columnCount(const QModelIndex &parent) const Q_DECL_OVERRIDE;
  QVariant data(const QModelIndex &index, int role) const Q_DECL_OVERRIDE;
  QVariant headerData(int section, Qt::Orientation orientation, int role) const
    Q_DECL_OVERRIDE;

public slots:
  void enable();
  void disable();

private:
  api::GvbVirtualMachine *m_vm;
  api::Array<api::GvbBinding> m_bindings;
};