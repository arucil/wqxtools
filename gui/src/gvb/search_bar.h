#pragma once

#include <QWidget>

class CodeEditor;
class QLineEdit;

class SearchBar: public QWidget {
  Q_OBJECT
public:
  SearchBar(CodeEditor *, QWidget * = nullptr);

public:
  void show(bool replace);
  void focus();
  bool isReplaceEnabled() const {
    return m_replaceBar->isVisible();
  }

private:
  void initUi();

private slots:
  void findPrevious();
  void findNext();
  void replace();
  void replaceAll();
  void setMatchCase(bool);
  void setWholeWord(bool);
  void setRegExp(bool);

private:
  CodeEditor *m_editor;
  QWidget *m_replaceBar;
  QLineEdit *m_searchEdit;
  QLineEdit *m_replaceEdit;
  bool m_matchCase;
  bool m_wholeWord;
  bool m_regexp;
};