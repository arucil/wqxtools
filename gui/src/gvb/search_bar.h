#pragma once

#include <QWidget>

class QLineEdit;
class QString;

class SearchBar: public QWidget {
  Q_OBJECT
public:
  SearchBar(QWidget * = nullptr);

public:
  void show(bool replace);
  void focus();
  bool isReplaceEnabled() const {
    return m_replace;
  }

private:
  void initUi();

signals:
  void findPrevious();
  void findNext();
  void replace();
  void replaceAll();
  void matchCaseChanged(bool);
  void wholeWordChanged(bool);
  void regExpChanged(bool);
  void searchTextChanged(const QString &);
  void replaceTextChanged(const QString &);

private:
  QWidget *m_replaceBar;
  QLineEdit *m_searchEdit;
  QLineEdit *m_replaceEdit;
  bool m_replace;
  bool m_searchTextChanged;
  bool m_replaceTextChanged;
};