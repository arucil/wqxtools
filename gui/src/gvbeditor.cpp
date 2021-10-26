#include "gvbeditor.h"
#include <QShowEvent>
#include <QTimer>
#include <QToolBar>
#include <QVBoxLayout>
#include <ScintillaEdit.h>

GvbEditor::GvbEditor(QWidget *parent) : QWidget(parent) { initUi(); }

GvbEditor::~GvbEditor() {}

void GvbEditor::initUi() {
  auto layout = new QVBoxLayout(this);
  auto toolbar = initToolBar();
  initEdit();

  layout->addWidget(toolbar);
  layout->addWidget(m_edit, 1);
  layout->setContentsMargins(0, 0, 0, 0);
  layout->setSpacing(0);
}

void GvbEditor::initEdit() {
  m_edit = new ScintillaEdit(this);

  QTimer::singleShot(0, [this] {
    auto defaultFontFamily = m_edit->styleFont(STYLE_DEFAULT);
    auto defaultFontSize = m_edit->styleSize(STYLE_DEFAULT);

    m_edit->styleSetFont(STYLE_LINENUMBER, defaultFontFamily.data());
    m_edit->styleSetSize(STYLE_LINENUMBER, defaultFontSize - 1);

    m_edit->styleSetFont(STYLE_DEFAULT, "WenQuXing");
    m_edit->styleSetSize(STYLE_DEFAULT, 13);

    m_edit->setMarginTypeN(2, SC_MARGIN_NUMBER);
    auto digitWidth = m_edit->textWidth(STYLE_LINENUMBER, "9");
    m_edit->setMarginWidthN(2, digitWidth);
  });
}

QToolBar *GvbEditor::initToolBar() {
  auto toolbar = new QToolBar;

  auto actSave =
      toolbar->addAction(QPixmap(":/assets/images/Save.svg"), "保存");
  connect(actSave, &QAction::triggered, this, &GvbEditor::save);

  toolbar->addSeparator();

  m_actStart = new QAction(QPixmap(":/assets/images/Run.svg"), "运行");
  toolbar->addAction(m_actStart);

  m_actStop = new QAction(QPixmap(":/assets/images/Stop.svg"), "停止");
  toolbar->addAction(m_actStop);

  toolbar->addSeparator();

  auto actFind =
      toolbar->addAction(QPixmap(":/assets/images/Find.svg"), "查找");
  connect(actFind, &QAction::triggered, this, &GvbEditor::find);

  auto actReplace =
      toolbar->addAction(QPixmap(":/assets/images/Replace.svg"), "替换");
  connect(actReplace, &QAction::triggered, this, &GvbEditor::replace);

  toolbar->addSeparator();

  auto actUndo =
      toolbar->addAction(QPixmap(":/assets/images/Undo.svg"), "撤销");
  connect(actUndo, &QAction::triggered, this, &GvbEditor::undo);

  auto actRedo =
      toolbar->addAction(QPixmap(":/assets/images/Redo.svg"), "重做");
  connect(actRedo, &QAction::triggered, this, &GvbEditor::redo);

  toolbar->addSeparator();

  auto actCopy =
      toolbar->addAction(QPixmap(":/assets/images/Copy.png"), "复制");
  connect(actCopy, &QAction::triggered, this, &GvbEditor::copy);

  auto actCut = toolbar->addAction(QPixmap(":/assets/images/Cut.svg"), "剪切");
  connect(actCut, &QAction::triggered, this, &GvbEditor::cut);

  auto actPaste =
      toolbar->addAction(QPixmap(":/assets/images/Paste.png"), "粘贴");
  connect(actPaste, &QAction::triggered, this, &GvbEditor::paste);

  return toolbar;
}

void GvbEditor::save() {
  // TODO
}

void GvbEditor::saveAs(const QString &) {
  // TODO
}

void GvbEditor::find() {
  // TODO
}

void GvbEditor::replace() {
  // TODO
}

void GvbEditor::cut() {
  // TODO
}

void GvbEditor::copy() {
  // TODO
}

void GvbEditor::paste() {
  // TODO
}

void GvbEditor::undo() {
  // TODO
}

void GvbEditor::redo() {
  // TODO
}