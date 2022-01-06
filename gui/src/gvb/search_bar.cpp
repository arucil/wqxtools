#include "search_bar.h"

#include <QCheckBox>
#include <QHBoxLayout>
#include <QLabel>
#include <QLineEdit>
#include <QPushButton>
#include <QVBoxLayout>

SearchBar::SearchBar(CodeEditor *editor, QWidget *parent) :
  QWidget(parent),
  m_editor(editor),
  m_matchCase(),
  m_wholeWord(),
  m_regexp() {
  initUi();
}

void SearchBar::initUi() {
  auto layout = new QVBoxLayout(this);

  auto searchLayout = new QHBoxLayout();
  searchLayout->setContentsMargins(0, 0, 0, 0);
  layout->addLayout(searchLayout);

  searchLayout->addWidget(new QLabel("查找", this));
  m_searchEdit = new QLineEdit(this);
  searchLayout->addWidget(m_searchEdit, 1);

  auto btnPrev = new QPushButton("上一个", this);
  searchLayout->addWidget(btnPrev);
  connect(btnPrev, &QPushButton::clicked, this, &SearchBar::findPrevious);

  auto btnNext = new QPushButton("下一个", this);
  searchLayout->addWidget(btnNext);
  connect(btnNext, &QPushButton::clicked, this, &SearchBar::findNext);

  m_replaceBar = new QWidget(this);
  layout->addWidget(m_replaceBar);
  m_replaceBar->hide();

  auto replaceLayout = new QHBoxLayout(m_replaceBar);
  replaceLayout->setContentsMargins(0, 0, 0, 0);
  replaceLayout->addWidget(new QLabel("替换", m_replaceBar));

  m_replaceEdit = new QLineEdit(m_replaceBar);
  replaceLayout->addWidget(m_replaceEdit, 1);

  auto btnRep = new QPushButton("替换", this);
  replaceLayout->addWidget(btnRep);
  connect(btnRep, &QPushButton::clicked, this, &SearchBar::replace);

  auto btnRepAll = new QPushButton("替换全部", this);
  replaceLayout->addWidget(btnRepAll);
  connect(btnRepAll, &QPushButton::clicked, this, &SearchBar::replaceAll);

  auto flags = new QWidget(this);
  auto flagLayout = new QHBoxLayout(flags);
  flagLayout->setContentsMargins(0, 0, 0, 0);
  layout->addWidget(flags);

  auto matchCase = new QCheckBox("匹配大小写", flags);
  flagLayout->addWidget(matchCase);
  connect(matchCase, &QCheckBox::clicked, this, &SearchBar::setMatchCase);

  auto wholeWord = new QCheckBox("全词匹配", flags);
  flagLayout->addWidget(wholeWord);
  connect(wholeWord, &QCheckBox::clicked, this, &SearchBar::setWholeWord);

  auto regexp = new QCheckBox("正则表达式", flags);
  flagLayout->addWidget(regexp);
  connect(regexp, &QCheckBox::clicked, this, &SearchBar::setRegExp);

  flagLayout->addStretch();
}

void SearchBar::show(bool replace) {
  QWidget::show();
  auto v = m_replaceBar->isVisible();
  // TODO margin？
  auto h = m_replaceBar->geometry().height();
  m_replaceBar->setVisible(replace);
  if (v) {
    setFixedHeight(height() - h);
  } else {
    setMinimumHeight(0);
    setMaximumHeight(QWIDGETSIZE_MAX);
  }
}

void SearchBar::focus() {
  m_searchEdit->setFocus();
}

void SearchBar::setMatchCase(bool b) {
  m_matchCase = b;
}

void SearchBar::setWholeWord(bool b) {
  m_wholeWord = b;
}

void SearchBar::setRegExp(bool b) {
  m_regexp = b;
}

void SearchBar::findPrevious() {
  // TODO
}

void SearchBar::findNext() {
  // TODO
}

void SearchBar::replace() {
  // TODO
}

void SearchBar::replaceAll() {
  // TODO
}