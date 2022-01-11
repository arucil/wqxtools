#pragma once

#include <QWidget>

class QEvent;
class QLabel;
class QShowEvent;

class EmojiSelector : public QWidget {
  Q_OBJECT
public:
  EmojiSelector(QWidget *parent = nullptr);

  void moveBeneath(QWidget *);

signals:
  void shown();

private slots:
  void clickedEmoji(QLabel *);
  void releasedEmoji(QLabel *);

protected:
  void changeEvent(QEvent *) override;
  void showEvent(QShowEvent *) override;

private:
  void initUi();
};