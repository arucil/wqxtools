#pragma once

#include "value.h"
#include "util.h"
#include <QAction>

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

public:
  BoolValue m_copyCutEnabled;
  BoolValue m_pasteEnabled;
  BoolValue m_undoEnabled;
  BoolValue m_redoEnabled;
  BoolValue m_dirty;
};

struct FileCapabilities {
  virtual ActionResult save(const QString &) = 0;
  virtual void create() = 0;

public:
  QAction *m_actSave;
};