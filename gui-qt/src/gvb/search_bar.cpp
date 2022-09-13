#include "search_bar.h"

#include <QCheckBox>
#include <QHBoxLayout>
#include <QLabel>
#include <QLineEdit>
#include <QPushButton>
#include <QVBoxLayout>

SearchBar::SearchBar(QWidget *parent) :
  QWidget(parent),
  m_replace(),
  m_searchTextChanged(),
  m_replaceTextChanged() {
  initUi();
}

void SearchBar::initUi() {
  auto layout = new QVBoxLayout(this);

  auto searchLayout = new QHBoxLayout();
  searchLayout->setContentsMargins({});
  layout->addLayout(searchLayout);

  searchLayout->addWidget(new QLabel("查找", this));
  m_searchEdit = new QLineEdit(this);
  connect(m_searchEdit, &QLineEdit::textChanged, this, [this] {
    m_searchTextChanged = true;
  });
  connect(m_searchEdit, &QLineEdit::editingFinished, this, [this] {
    if (m_searchTextChanged) {
      emit searchTextChanged(m_searchEdit->text());
      m_searchTextChanged = false;
    }
    if (m_searchEdit->hasFocus()) {
      emit findNext();
    }
  });
  searchLayout->addWidget(m_searchEdit, 1);

  auto btnNext = new QPushButton("下一个", this);
  searchLayout->addWidget(btnNext);
  connect(btnNext, &QPushButton::clicked, this, &SearchBar::findNext);

  auto btnPrev = new QPushButton("上一个", this);
  searchLayout->addWidget(btnPrev);
  connect(btnPrev, &QPushButton::clicked, this, &SearchBar::findPrevious);

  m_replaceBar = new QWidget(this);
  layout->addWidget(m_replaceBar);

  auto replaceLayout = new QHBoxLayout(m_replaceBar);
  replaceLayout->setContentsMargins({});
  replaceLayout->addWidget(new QLabel("替换", m_replaceBar));

  m_replaceEdit = new QLineEdit(m_replaceBar);
  connect(m_replaceEdit, &QLineEdit::textChanged, this, [this] {
    m_replaceTextChanged = true;
  });
  connect(m_replaceEdit, &QLineEdit::editingFinished, this, [this] {
    if (m_replaceTextChanged) {
      emit replaceTextChanged(m_replaceEdit->text());
    }
  });
  replaceLayout->addWidget(m_replaceEdit, 1);

  auto btnRep = new QPushButton("替换", this);
  replaceLayout->addWidget(btnRep);
  connect(btnRep, &QPushButton::clicked, this, &SearchBar::replace);

  auto btnRepAll = new QPushButton("替换全部", this);
  replaceLayout->addWidget(btnRepAll);
  connect(btnRepAll, &QPushButton::clicked, this, &SearchBar::replaceAll);

  auto flags = new QWidget(this);
  auto flagLayout = new QHBoxLayout(flags);
  flagLayout->setContentsMargins({});
  layout->addWidget(flags);

  auto matchCase = new QCheckBox("匹配大小写", flags);
  flagLayout->addWidget(matchCase);
  connect(matchCase, &QCheckBox::clicked, this, &SearchBar::matchCaseChanged);

  auto wholeWord = new QCheckBox("全词匹配", flags);
  flagLayout->addWidget(wholeWord);
  connect(wholeWord, &QCheckBox::clicked, this, &SearchBar::wholeWordChanged);

  auto regexp = new QCheckBox("正则表达式", flags);
  flagLayout->addWidget(regexp);
  connect(regexp, &QCheckBox::clicked, this, &SearchBar::regExpChanged);

  flagLayout->addStretch();
}

void SearchBar::show(bool replace) {
  QWidget::show();
  auto h = m_replaceBar->height();
  m_replaceBar->setVisible(replace);
  if (m_replace && !replace) {
    setFixedHeight(height() - h - layout()->spacing());
  } else {
    // unset fixed height
    setMinimumHeight(0);
    setMaximumHeight(QWIDGETSIZE_MAX);
  }
  m_replace = replace;
}

void SearchBar::focus() {
  m_searchEdit->setFocus();
  m_searchEdit->selectAll();
}

bool SearchBar::hasFocus() const {
  return m_searchEdit->hasFocus() || m_replaceEdit->hasFocus();
}