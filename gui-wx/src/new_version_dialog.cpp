#include "new_version_dialog.h"

#include <wx/hyperlink.h>

NewVersionDialog::NewVersionDialog(
  wxWindow *parent,
  const wxString &version,
  const wxString &desc,
  const wxString &url) :
  wxDialog(parent, wxID_ANY, wxT("新版本")) {
  SetSizeHints(wxDefaultSize, wxDefaultSize);

  auto sizer = new wxBoxSizer(wxVERTICAL);

  auto verText = new wxStaticText(
    this,
    wxID_ANY,
    wxString::Format(wxT("版本：%s\n"), version));
  sizer->Add(verText, 0, wxALL, 10);

  auto descText = new wxStaticText(this, wxID_ANY, desc);
  sizer->Add(descText, 0, wxALL, 10);

  auto link =
    new wxHyperlinkCtrl(this, wxID_ANY, wxT("点击链接下载新版本"), url);
  sizer->Add(link, 0, wxALL, 10);

  SetSizer(sizer);
  Layout();
  Center();
}