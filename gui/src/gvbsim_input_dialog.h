#pragma once

#include <QDialog>
#include <QString>
#include <QVector>
#include <cstdint>
#include <variant>

#include "api.h"

class QShowEvent;
class QKeyEvent;

typedef api::Array<std::uint8_t> ByteString;
typedef api::GvbInputFuncBody *FuncBody;
typedef std::variant<std::int16_t, api::GvbReal, ByteString, FuncBody>
  InputField;

class GvbSimInputDialog: public QDialog {
  Q_OBJECT

public:
  GvbSimInputDialog(
    QWidget *,
    const api::GvbVirtualMachine *,
    const api::GvbExecResult::KeyboardInput_Body &);
  ~GvbSimInputDialog();

  QVector<api::GvbKeyboardInput> inputData();

protected:
  void keyPressEvent(QKeyEvent *) Q_DECL_OVERRIDE;
  void reject() Q_DECL_OVERRIDE;

signals:
  void validateAll();

private:
  void initUi(const api::GvbExecResult::KeyboardInput_Body &);
  void endValidateAll();

private slots:
  void fieldValidated(bool);
  void startValidateAll();

private:
  const api::GvbVirtualMachine *m_vm;
  QVector<InputField> m_input;
  bool m_validateAll;
  size_t m_validatedFields;
  size_t m_validateOkFields;
  bool m_rejected;
};