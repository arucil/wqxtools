#pragma once

#include <wx/wx.h>
#include <wx/html/helpctrl.h>
#include <optional>

class MainWindow: public wxFrame {
public:
  MainWindow();

private:
  void initUi();
  void initMenu();

private:
  void onHelp(wxCommandEvent &);
  void onAbout(wxCommandEvent &);

private:
  std::optional<wxHtmlHelpController> m_helpCtrl;
};