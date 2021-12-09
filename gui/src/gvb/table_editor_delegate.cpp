#include "table_editor_delegate.h"
#include "table_editor_model.h"

TableEditorDelegate::TableEditorDelegate(QObject *parent) :
  QStyledItemDelegate(parent) {}

TableEditorDelegate::~TableEditorDelegate() {}

QWidget *TableEditorDelegate::createEditor(
  QWidget *parent,
  const QStyleOptionViewItem &,
  const QModelIndex &index) const {
  return qobject_cast<const TableEditorModel *>(index.model())
    ->createEditor(parent, index);
}

void TableEditorDelegate::setEditorData(QWidget *editor, const QModelIndex &index)
  const {
  qobject_cast<const TableEditorModel *>(index.model())->setEditorData(editor, index);
}

void TableEditorDelegate::setModelData(
  QWidget *editor,
  QAbstractItemModel *model,
  const QModelIndex &index) const {
  qobject_cast<TableEditorModel *>(model)->setData(editor, index);
}

void TableEditorDelegate::updateEditorGeometry(
  QWidget *editor,
  const QStyleOptionViewItem &option,
  const QModelIndex & /* index */) const {
  editor->setGeometry(option.rect);
}