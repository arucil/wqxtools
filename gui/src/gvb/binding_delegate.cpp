#include "binding_delegate.h"

#include "binding_model.h"

BindingDelegate::BindingDelegate(QObject *parent) :
  QStyledItemDelegate(parent) {}

BindingDelegate::~BindingDelegate() {}

QWidget *BindingDelegate::createEditor(
  QWidget *parent,
  const QStyleOptionViewItem &,
  const QModelIndex &index) const {
  return qobject_cast<const BindingModel *>(index.model())
    ->createEditor(parent, index);
}

void BindingDelegate::setEditorData(QWidget *editor, const QModelIndex &index)
  const {
  qobject_cast<const BindingModel *>(index.model())->setEditorData(editor, index);
}

void BindingDelegate::setModelData(
  QWidget *editor,
  QAbstractItemModel *model,
  const QModelIndex &index) const {
  qobject_cast<BindingModel *>(model)->setData(editor, index);
}

void BindingDelegate::updateEditorGeometry(
  QWidget *editor,
  const QStyleOptionViewItem &option,
  const QModelIndex & /* index */) const {
  editor->setGeometry(option.rect);
}