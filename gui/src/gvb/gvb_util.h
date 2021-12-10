#pragma once

#include <QString>
#include <optional>

#include "api.h"

class QWidget;

QString arrayBindingName(const api::GvbBinding::Array_Body &);

QString arraySubsToString(
  const api::Utf8Str &name,
  const api::Array<std::uint16_t> &subs);

std::optional<api::GvbValue> inputString(
  QWidget *parent,
  const api::GvbVirtualMachine *,
  const QString &title,
  const api::Array<std::uint8_t> &initial);