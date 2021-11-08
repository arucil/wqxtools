#pragma once

#include "util.h"
#include "value.h"
#include <QAction>
#include <optional>

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

// succeeded: new path
// failed: error message (optional)
// cancelled
using SaveResult = std::variant<QString, std::optional<QString>, Unit>;

struct FileCapabilities {
  virtual SaveResult save(const QString &) = 0;
  virtual void create() = 0;

public:
  QAction *m_actSave;
};