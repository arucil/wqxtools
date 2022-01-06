#pragma once

#include <optional>

#include "util.h"
#include "value.h"

class Action;
class QAction;
class QObject;
class QString;
class QState;

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
  virtual const char *defaultExt() const = 0;

public:
  QAction *m_actSave;
};

struct ProgramCapabilities {
public:
  QState *m_stStarted;
  QState *m_stPaused;
  QState *m_stStopped;
  QAction *m_actStart;
  QAction *m_actStop;
};