#pragma once

#include <wx/wx.h>

class MainWindow: public wxFrame {
public:
  MainWindow();

private:
  void initUi();
  void initMenu();

private:
  void onHelp(wxCommandEvent &);
  void onAbout(wxCommandEvent &);
};