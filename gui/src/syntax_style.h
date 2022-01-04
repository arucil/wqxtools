#pragma once

#include <QColor>
#include <QHash>
#include <QString>
#include <variant>

class QFile;

/// https://www.scintilla.org/ScintillaDoc.html#SCI_INDICSETSTYLE
enum class UnderlineStyle {
  Plain = 0,
  Squiggle = 1,
  TT = 2,
  Diagonal = 3,
  Strike = 4,
  Hidden = 5,
  Box = 6,
  RoundBox = 7,
  Dash = 9,
  Dots = 10,
};

struct CharFormat {
  bool bold;
  bool italic;
  QColor background;
  QColor foreground;
  QColor underlineColor;
  UnderlineStyle underlineStyle;
};

class SyntaxStyle {
public:
  static std::variant<QString, SyntaxStyle> load(QFile &xml);

  const CharFormat *getFormat(const QString &name) const;

private:
  SyntaxStyle(QHash<QString, CharFormat>);

private:
  QHash<QString, CharFormat> m_formats;
};