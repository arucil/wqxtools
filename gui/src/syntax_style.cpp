#include "syntax_style.h"

#include <QXmlStreamReader>
#include <utility>
#include <QFile>

const CharFormat *SyntaxStyle::getFormat(const QString &name) const {
  auto it = m_formats.constFind(name);
  if (it == m_formats.constEnd()) {
    return nullptr;
  } else {
    return &*it;
  }
}

std::variant<QString, SyntaxStyle> SyntaxStyle::load(QFile &xml) {
  QXmlStreamReader reader(&xml);
  QHash<QString, CharFormat> formats;

  while (!reader.atEnd() && !reader.hasError()) {
    auto token = reader.readNext();

    if (token == QXmlStreamReader::StartElement) {
      if (reader.name() == u"style-scheme") {
        if (reader.attributes().hasAttribute("name")) {
          //m_name = reader.attributes().value("name").toString();
        }
      } else if (reader.name() == u"style") {
        auto attributes = reader.attributes();

        auto name = attributes.value("name");

        CharFormat format;

        if (attributes.hasAttribute("background")) {
          format.background = QColor(attributes.value("background").toString());
        }

        if (attributes.hasAttribute("foreground")) {
          format.foreground = QColor(attributes.value("foreground").toString());
        }

        if (
          attributes.hasAttribute("bold")
          && attributes.value("bold") == u"true") {
          format.bold = true;
        }

        if (
          attributes.hasAttribute("italic")
          && attributes.value("italic") == u"true") {
          format.italic = true;
        }

        if (attributes.hasAttribute("underlineStyle")) {
          auto underline = attributes.value("underlineStyle");

          if (underline == u"SingleUnderline") {
            format.underlineStyle = UnderlineStyle::Plain;
          } else if (underline == u"DashUnderline") {
            format.underlineStyle = UnderlineStyle::Dash;
          } else if (underline == u"DotLine") {
            format.underlineStyle = UnderlineStyle::Dots;
          } else if (underline == u"WaveUnderline") {
            format.underlineStyle = UnderlineStyle::Squiggle;
          } else {
            return QString("unknown underline style: %1").arg(underline);
          }
        }

        if (attributes.hasAttribute("underlineColor")) {
          auto color = attributes.value("underlineColor");
          format.underlineColor = QColor(color.toString());
        }

        formats.insert(name.toString(), format);
      }
    }
  }

  if (reader.hasError()) {
    return QString("XML parse error: %1").arg(reader.errorString());
  }

  return SyntaxStyle(std::move(formats));
}

SyntaxStyle::SyntaxStyle(QHash<QString, CharFormat> formats) :
  m_formats(formats) {}