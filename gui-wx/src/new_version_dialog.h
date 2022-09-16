#pragma once

#include <wx/wx.h>

class NewVersionDialog: public wxDialog {
public:
  NewVersionDialog(
    wxWindow *parent,
    const wxString &version,
    const wxString &desc,
    const wxString &url);
  ~NewVersionDialog() {}
};