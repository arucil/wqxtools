#pragma once

#include "value.h"

class QObject;
class QString;

struct EditCapabilities {
  virtual void copy() = 0;
  virtual void cut() = 0;
  virtual void paste() = 0;

  virtual void undo() = 0;
  virtual void redo() = 0;

  virtual void find() = 0;
  virtual void replace() = 0;

  EditCapabilities() : m_dirty(false) {}

public:
  BoolValue m_copyCutEnabled;
  BoolValue m_pasteEnabled;
  BoolValue m_undoEnabled;
  BoolValue m_redoEnabled;
  bool m_dirty;
};

struct FileCapabilities {
  virtual void save() = 0;
  virtual void saveAs(const QString &) = 0;
  virtual void create() = 0;
};