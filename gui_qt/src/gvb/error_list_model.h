#pragma once

#include <QAbstractTableModel>
#include <QVector>

#include "code_editor.h"

class ErrorListModel: public QAbstractTableModel {
  Q_OBJECT
public:
  ErrorListModel(const QVector<Diagnostic> *, QObject *parent = nullptr);

public:
  int rowCount(const QModelIndex &parent) const override;
  int columnCount(const QModelIndex &parent) const override;
  QVariant data(const QModelIndex &index, int role) const override;
  QVariant
  headerData(int section, Qt::Orientation orientation, int role) const override;

public slots:
  void diagnosticsChanged(int len);

private:
  const QVector<Diagnostic> *m_diagnostics;
  int m_len;
};