#include <QVBoxLayout>
#include <QToolBar>
#include "gvbeditor.h"

GvbEditor::GvbEditor(QWidget *parent) : QWidget(parent) {
  auto layout = new QVBoxLayout(this);

  auto toolbar = new QToolBar;
  toolbar->addAction();

  layout->addWidget(toolbar);

  // 保存,
  // ---
  // 运行/暂停
  // 停止
  // ---
  // 查找
  // 替换
  // ---
  // 撤销
  // 重做
  // ---
  // 复制
  // 剪切
  // 粘贴
}