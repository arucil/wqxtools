#pragma once

#include <QObject>

enum class MessageType {
  Info,
  Error,
};

class MessageBus : public QObject {
  Q_OBJECT
public:
  MessageBus(const MessageBus &) = delete;
  MessageBus &operator=(const MessageBus &) = delete;

  static MessageBus *instance();

signals:
  void newMessage(const QString &, int ms, MessageType);

public slots:
  void postMessage(const QString &, int ms, MessageType);

private:
  MessageBus();
};

Q_DECLARE_METATYPE(MessageType);