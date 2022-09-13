#pragma once

#include <QAbstractTableModel>

class TableEditorModel: public QAbstractTableModel {
  Q_OBJECT

public:
  virtual QWidget *
  createEditor(QWidget *parent, const QModelIndex &index) const = 0;
  virtual void setEditorData(QWidget *editor, const QModelIndex &) const = 0;
  virtual void setData(QWidget *editor, const QModelIndex &) = 0;
};