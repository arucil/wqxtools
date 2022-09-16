#pragma once

#include <wx/wx.h>
#include <optional>

std::optional<wxMemoryBuffer> readAll(wxInputStream &);

void showErrorMessage(const wxString &message, wxWindow *parent = nullptr);