#pragma once

#include <QTextBrowser>

class QHelpEngine;

class HelpBrowser : public QTextBrowser {
  Q_OBJECT

public:
  HelpBrowser(QHelpEngine *, QWidget *parent = nullptr);

protected:
  QVariant loadResource(int type, const QUrl &) override;

private:
  QHelpEngine *m_helpEngine;
};