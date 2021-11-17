#pragma once

#include "util.h"
#include "value.h"
#include "action.h"
#include <QAction>
#include <optional>

class QObject;
class QString;

struct EditCapabilities {
  Action *m_actCopy;
  Action *m_actCut;
  Action *m_actPaste;
  Action *m_actUndo;
  Action *m_actRedo;
  Action *m_actFind;
  Action *m_actReplace;
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

struct ProgramCapabilities {
public:
  BoolValue m_isPaused;
  BoolValue m_started;
  Action *m_actStart;
  Action *m_actStop;
};