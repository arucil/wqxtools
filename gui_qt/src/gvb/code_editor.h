#pragma once

#include <api.h>

#include <QString>
#include <QVector>
#include <interval-tree/interval_tree.hpp>
#include <optional>
#include <string>

class SyntaxStyle;
class QPoint;
class QContextMenuEvent;
class QKeyEvent;

struct Diagnostic {
  size_t line;
  size_t start;
  size_t end;
  api::GvbSeverity severity;
  QString message;
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
    return {qMin(low_, other.low_), qMax(high_, other.high_)};
  }

private:
  value_type low_;
  value_type high_;

public:
  size_t index;
};

enum class TextChangeKind {
  InsertText,
  DeleteText,
};

struct TextChange {
  TextChangeKind kind;
  size_t position;
  const char *text;
  size_t length;
};

class CodeEditor: public ScintillaEdit {
  Q_OBJECT

public:
  CodeEditor(QWidget *parent = nullptr);

  const QVector<Diagnostic> &diagnostics() {
    return m_diagnostics;
  }

protected:
  void contextMenuEvent(QContextMenuEvent *) override;
  void keyPressEvent(QKeyEvent *) override;

private:
  void adjustLineNumberMarginWidth();
  void showDiagnostics(size_t pos, const QPoint &);

signals:
  void cursorPositionChanged(size_t);
  void dirtyChanged(bool isDirty);
  void textChanged(const TextChange &);
  void selectionChanged(bool nonempty);
  void fileDropped(const QString &path);
  void contextMenu(const QPoint &localPos);
  void escape();

public slots:
  void setDiagnostics(QVector<Diagnostic>);
  void setRuntimeError(const Diagnostic &);
  void clearRuntimeError();
  void setStyle(const SyntaxStyle *);
  void setFontSize(unsigned);
  void setSearchMatchCase(bool);
  void setSearchWholeWord(bool);
  void setSearchRegExp(bool);
  bool findNext();
  void findPrevious();
  void replace();
  void replaceAll();
  void setSearchText(const QString &);
  void setReplaceText(const QString &);

private slots:
  void notified(Scintilla::NotificationData *);

private:
  QVector<Diagnostic> m_diagnostics;
  std::optional<Diagnostic> m_runtimeError;
  lib_interval_tree::interval_tree<Range> m_diagRanges;
  bool m_dirty;
  bool m_braceHilit;
  std::string m_searchText;
  std::string m_replaceText;
};
