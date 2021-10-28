#include "gvbeditor.h"
#include "util.h"
#include <QApplication>
#include <QByteArray>
#include <QMessageBox>
#include <QTimer>
#include <QToolBar>
#include <QVBoxLayout>
#include <ScintillaEdit.h>

GvbEditor::GvbEditor(QWidget *parent) : QWidget(parent) {
  initUi();

  QTimer::singleShot(0, [this] {
    m_pasteEnabled.setValue(true);
    m_undoEnabled.setValue(false);
    m_redoEnabled.setValue(false);
    m_copyCutEnabled.setValue(true);
  });

  QTimer::singleShot(0, [this] {
    auto name = QString("小爬虫.BAS");
    auto result = gvb::load_document(name.utf16(), name.size());
    if (result.tag == gvb::Either<gvb::CString, gvb::Document *>::Tag::Left) {
      auto msg = result.left._0;
      auto msgbox =
          new QMessageBox(QMessageBox::Icon::Critical, "打开失败",
                          QString::fromUtf8(QByteArray(msg.data, msg.len)),
                          QMessageBox::StandardButton::Ok, getMainWindow());
      gvb::destroy_string(msg);
      msgbox->show();
    } else {
      m_doc = result.right._0;

      auto text = gvb::document_text(m_doc);

      m_edit->setText(text.data);
    }
  });
}

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

  auto defaultFontFamily = m_edit->styleFont(STYLE_DEFAULT);
  // auto defaultFontSize = m_edit->styleSize(STYLE_DEFAULT);

  m_edit->styleSetFont(STYLE_LINENUMBER, defaultFontFamily.data());
  // m_edit->styleSetSize(STYLE_LINENUMBER, defaultFontSize);
  m_edit->styleSetFore(STYLE_LINENUMBER, 0xff'a4'72'62);
  m_edit->styleSetBack(STYLE_LINENUMBER, 0xff'ef'ef'ef);

  m_edit->styleSetFont(STYLE_DEFAULT, "WenQuXing");
  m_edit->styleSetSize(STYLE_DEFAULT, 12);

  m_edit->styleSetFont(0, "WenQuXing");
  m_edit->styleSetSize(0, 12);

  m_edit->setMarginTypeN(2, SC_MARGIN_NUMBER);
  auto digitWidth = m_edit->textWidth(STYLE_LINENUMBER, "9");
  m_edit->setMarginWidthN(2, digitWidth);

  m_edit->setModEventMask(SC_MOD_INSERTTEXT | SC_MOD_DELETETEXT);

  m_edit->setElementColour(SC_ELEMENT_CARET_LINE_BACK, 0xff'f6'ee'e0);
  m_edit->setCaretLineVisibleAlways(true);
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
  connect(&m_undoEnabled, &BoolValue::changed, actUndo, &QAction::setEnabled);

  auto actRedo =
      toolbar->addAction(QPixmap(":/assets/images/Redo.svg"), "重做");
  connect(actRedo, &QAction::triggered, this, &GvbEditor::redo);
  connect(&m_redoEnabled, &BoolValue::changed, actRedo, &QAction::setEnabled);

  toolbar->addSeparator();

  auto actCopy =
      toolbar->addAction(QPixmap(":/assets/images/Copy.png"), "复制");
  connect(actCopy, &QAction::triggered, this, &GvbEditor::copy);
  connect(&m_copyCutEnabled, &BoolValue::changed, actCopy,
          &QAction::setEnabled);

  auto actCut = toolbar->addAction(QPixmap(":/assets/images/Cut.svg"), "剪切");
  connect(actCut, &QAction::triggered, this, &GvbEditor::cut);
  connect(&m_copyCutEnabled, &BoolValue::changed, actCut, &QAction::setEnabled);

  auto actPaste =
      toolbar->addAction(QPixmap(":/assets/images/Paste.png"), "粘贴");
  connect(actPaste, &QAction::triggered, this, &GvbEditor::paste);
  connect(&m_pasteEnabled, &BoolValue::changed, actPaste, &QAction::setEnabled);

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

void GvbEditor::cut() { m_edit->cut(); }

void GvbEditor::copy() { m_edit->copy(); }

void GvbEditor::paste() { m_edit->paste(); }

void GvbEditor::undo() { m_edit->undo(); }

void GvbEditor::redo() { m_edit->redo(); }