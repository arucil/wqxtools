#pragma once

#include <QStateMachine>
#include <QStatusBar>
#include <QWidget>
#include <algorithm>
#include <string>
#include <variant>
#include <vector>

#include "api.h"
#include "capability.h"
#include "interval-tree/interval_tree.hpp"
#include "tool.h"

class QAction;
class QToolBar;
class ScintillaEdit;
class QShowEvent;
class QTimerEvent;
class QLabel;
class GvbSimWindow;
class QCloseEvent;

namespace Scintilla {
class NotificationData;
}

struct InsertText {
  size_t pos;
  std::string str;
};

struct DeleteText {
  size_t pos;
  size_t len;
};

struct Diagnostic {
  size_t line;
  size_t start;
  size_t end;
  api::GvbSeverity severity;
  std::string message;
};

struct Range {
public:
  using value_type = size_t;
  using interval_kind = lib_interval_tree::closed;

  constexpr Range(value_type low, value_type high) :
    low_ {low},
    high_ {high},
    index(0) {
    assert(low <= high);
  }

  /**
   *  Returns if both intervals equal.
   */
  friend bool operator==(Range const &lhs, Range const &other) {
    return lhs.low_ == other.low_ && lhs.high_ == other.high_;
  }

  /**
   *  Returns if both intervals are different.
   */
  friend bool operator!=(Range const &lhs, Range const &other) {
    return lhs.low_ != other.low_ || lhs.high_ != other.high_;
  }

  /**
   *  Returns the lower bound of the interval
   */
  value_type low() const {
    return low_;
  }

  /**
   *  Returns the upper bound of the interval
   */
  value_type high() const {
    return high_;
  }

  /**
   *  Returns whether the intervals overlap.
   *  For when both intervals are closed.
   */
  bool overlaps(value_type l, value_type h) const {
    return low_ <= h && l <= high_;
  }

  /**
   *  Returns whether the intervals overlap, excluding border.
   *  For when at least one interval is open (l&r).
   */
  bool overlaps_exclusive(value_type l, value_type h) const {
    return low_ < h && l < high_;
  }

  /**
   *  Returns whether the intervals overlap
   */
  bool overlaps(Range const &other) const {
    return overlaps(other.low_, other.high_);
  }

  /**
   *  Returns whether the intervals overlap, excluding border.
   */
  bool overlaps_exclusive(Range const &other) const {
    return overlaps_exclusive(other.low_, other.high_);
  }

  /**
   *  Returns whether the given value is in this.
   */
  bool within(value_type value) const {
    return interval_kind::within(low_, high_, value);
  }

  /**
   *  Returns whether the given interval is in this.
   */
  bool within(Range const &other) const {
    return low_ <= other.low_ && high_ >= other.high_;
  }

  /**
   *  Calculates the distance between the two intervals.
   *  Overlapping intervals have 0 distance.
   */
  value_type operator-(Range const &other) const {
    if (overlaps(other))
      return 0;
    if (high_ < other.low_)
      return other.low_ - high_;
    else
      return low_ - other.high_;
  }

  /**
   *  Returns the size of the interval.
   */
  value_type size() const {
    return high_ - low_;
  }

  /**
   *  Creates a new interval from this and other, that contains both intervals
   * and whatever is between.
   */
  Range join(Range const &other) const {
    return {std::min(low_, other.low_), std::max(high_, other.high_)};
  }

private:
  value_type low_;
  value_type high_;

public:
  size_t index;
};

struct GvbEditor:
  Tool,
  EditCapabilities,
  FileCapabilities,
  ProgramCapabilities {
private:
  Q_OBJECT

public:
  GvbEditor(QWidget *parent = nullptr);
  ~GvbEditor();

private:
  void initUi();
  void initEdit();
  void initStateMachine();
  QToolBar *initToolBar();
  QStatusBar *initStatusBar();
  void computeDiagnostics();
  void updateStartAction(QState *);

signals:
  void updateDiagnostics(std::vector<Diagnostic>);
  void start();
  void pause();
  void cont();
  void stop();

public slots:
  SaveResult save(const QString &) override;
  void create();
  void find();
  void replace();
  void cut();
  void copy();
  void paste();
  void undo();
  void redo();
  LoadResult load(const QString &) override;
  bool canLoad(const QString &) const override;
  QSize preferredWindowSize() const override;
  void tryStartPause(QWidget *sender);

private slots:
  void notified(Scintilla::NotificationData *);
  void diagnosticsUpdated(std::vector<Diagnostic>);
  void modified();

private:
  ScintillaEdit *m_edit;
  api::GvbDocument *m_doc;
  bool m_textLoaded;
  bool m_timerModify;
  std::vector<std::variant<InsertText, DeleteText>> m_edits;
  std::vector<Diagnostic> m_diagnostics;
  lib_interval_tree::interval_tree<Range> m_diagRanges;
  SizeValue m_curPos;
  QStateMachine m_stateMachine;
  GvbSimWindow *m_gvbsim;
  QString m_filePath;
};

Q_DECLARE_METATYPE(std::vector<Diagnostic>);