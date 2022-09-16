#pragma once

#include <wx/wx.h>
#include <wx/html/helpctrl.h>
#include <optional>

class MainWindow: public wxFrame {
public:
  MainWindow(const wxString &filePath = wxString());

private:
  void initUi();
  void initMenu();

private:
  void onHelp(wxCommandEvent &);
  void onAbout(wxCommandEvent &);

  void setFileLoaded(bool);
  void setOpenFile(const wxString &path);
  void updateTitle();
  void checkNewVersion(bool isManual);
  void notifyNewVersion(const wxString &tag);

private:
  std::optional<wxHtmlHelpController> m_helpCtrl;
  wxString m_openFilePath;
};