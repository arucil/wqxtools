#include "notification.h"

#include <wx/notifmsg.h>

static wxNotificationMessage notifMsg;

void showNotification(
  const wxString &message,
  NotificationType type,
  int timeoutSec) {
  notifMsg.SetMessage(message);
  switch (type) {
    case NotificationType::Information:
      notifMsg.SetFlags(wxICON_INFORMATION);
      break;
    case NotificationType::Warning:
      notifMsg.SetFlags(wxICON_WARNING);
      break;
    case NotificationType::Error:
      notifMsg.SetFlags(wxICON_ERROR);
      break;
  }
  // TODO warning and error will not be automatically hidden on GTK, use wxTimer instead.
  if (timeoutSec == 0) {
    notifMsg.Show();
  } else {
    notifMsg.Show(timeoutSec);
  }
}

void hideNotification() {
  notifMsg.Close();
}