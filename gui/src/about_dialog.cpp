#include "about_dialog.h"

#include <QDialogButtonBox>
#include <QLabel>
#include <QVBoxLayout>

AboutDialog::AboutDialog(QWidget *parent) : QDialog(parent) {
  auto layout = new QVBoxLayout(this);
  auto about = new QLabel(
    "<p>作者：arucil</p>"
    "<p>Github仓库：<a>https://github.com/arucil/wqxtools</a></p>"
    "<p>开源协议：MIT License</p>"
    "<br/>"
    R"(<p>GVBASIC 编辑器的图标来源：<br>
    Noto Emoji: <a>https://github.com/googlefonts/noto-emoji</a><br>
    Elementary OS Icons: <a>https://github.com/elementary/icons</a>
    </p>)");
  about->setTextInteractionFlags(Qt::TextSelectableByMouse);
  about->setAlignment(Qt::AlignLeft | Qt::AlignVCenter);
  about->setContentsMargins(15, 15, 15, 15);
  about->setCursor(Qt::IBeamCursor);
  layout->addWidget(about);
  auto buttons = new QDialogButtonBox(QDialogButtonBox::Ok);
  connect(buttons, &QDialogButtonBox::accepted, this, &QDialog::accept);
  layout->addWidget(buttons);
  setWindowTitle("关于 WQX 工具箱");
  adjustSize();
}