#pragma once

#include <wx/wx.h>

enum class NotificationType {
  Information,
  Warning,
  Error,
};

void showNotification(const wxString &, NotificationType, int timeoutSec = 0);

void hideNotification();