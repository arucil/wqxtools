#include "message_bus.h"

MessageBus::MessageBus() {
  qRegisterMetaType<MessageType>();
}

MessageBus *MessageBus::instance() {
  static MessageBus inst;
  return &inst;
}

void MessageBus::postMessage(const QString &text, int ms, MessageType type) {
  emit newMessage(text, ms, type);
}