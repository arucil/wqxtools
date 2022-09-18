#include "error_list_model.h"

#include <QIcon>
#include <QSize>

ErrorListModel::ErrorListModel(
  const QVector<Diagnostic> *diags,
  QObject *parent) :
  QAbstractTableModel(parent),
  m_diagnostics(diags),
  m_len(diags->size()) {}

int ErrorListModel::rowCount(const QModelIndex &) const {
  return m_len;
}

int ErrorListModel::columnCount(const QModelIndex &) const {
  return 3;
}

QVariant ErrorListModel::data(const QModelIndex &index, int role) const {
  switch (index.column()) {
    case 0:
      if (role == Qt::DecorationRole && index.row() < m_diagnostics->size()) {
        switch ((*m_diagnostics)[index.row()].severity) {
          case api::GvbSeverity::Warning:
            return QIcon(QPixmap(":/images/Warning.svg"));
          case api::GvbSeverity::Error:
            return QIcon(QPixmap(":/images/Error.svg"));
        }
      }
      break;
    case 1:
      if (
        (role == Qt::DisplayRole || role == Qt::ToolTipRole)
        && index.row() < m_diagnostics->size()) {
        return (*m_diagnostics)[index.row()].message;
      }
      break;
    case 2:
      if (role == Qt::DisplayRole && index.row() < m_diagnostics->size()) {
        return QString("第 %1 行").arg((*m_diagnostics)[index.row()].line + 1);
      } else if (role == Qt::TextAlignmentRole) {
        return Qt::AlignCenter;
      }
      break;
  }
  return {};
}

QVariant ErrorListModel::headerData(
  int section,
  Qt::Orientation orientation,
  int role) const {
  if (orientation == Qt::Horizontal) {
    if (role == Qt::DisplayRole) {
      switch (section) {
        case 1:
          return "问题";
        case 2:
          return "位置";
      }
    }
  }
  return {};
}

void ErrorListModel::diagnosticsChanged(int len) {
  if (m_len > len) {
    beginRemoveRows(QModelIndex(), len, m_len - 1);
    endRemoveRows();
  } else if (m_len < len) {
    beginInsertRows(QModelIndex(), m_len, len - 1);
    endInsertRows();
  }
  emit dataChanged(index(0, 0), index(len - 1, 2));
  m_len = len;
}