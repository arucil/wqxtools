#pragma once

#include <QStyledItemDelegate>

class BindingDelegate: public QStyledItemDelegate {
  Q_OBJECT

public:
  BindingDelegate(QObject *parent = nullptr);
  ~BindingDelegate();

  QWidget *createEditor(
    QWidget *parent,
    const QStyleOptionViewItem &option,
    const QModelIndex &index) const override;

  void setEditorData(QWidget *editor, const QModelIndex &index) const override;
  void setModelData(
    QWidget *editor,
    QAbstractItemModel *model,
    const QModelIndex &index) const override;

  void updateEditorGeometry(
    QWidget *editor,
    const QStyleOptionViewItem &option,
    const QModelIndex &index) const override;
};