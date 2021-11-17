#include "action.h"
#include <QEvent>

Action::Action(QObject *parent) : QAction(parent) {}

Action::Action(const QString &text, QObject *parent) : QAction(text, parent) {}

Action::Action(const QIcon &icon, const QString &text, QObject *parent)
    : QAction(icon, text, parent) {}

bool Action::event(QEvent *ev) {
  auto result = QAction::event(ev);
  if (ev->type() == QEvent::ActionChanged) {
    emit(enabledChanged(isEnabled()));
    return true;
  }
  return result;
}
