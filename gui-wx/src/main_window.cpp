#include "main_window.h"
#include <wx/notifmsg.h>

#include <wx/aboutdlg.h>
#include <wx/xrc/xmlres.h>

#include "api.h"

enum {
  ID_Menu_Start,
  ID_Menu_Stop,
  ID_Menu_ReloadConfig,
  ID_Menu_CheckVersion,
};

MainWindow::MainWindow() :
  wxFrame(
    nullptr,
    wxID_ANY,
    wxT("文曲星工具箱"),
    wxDefaultPosition,
    wxSize(400, 340)) {
  Center();
  // TODO drag'n'drop
  initUi();
}

void MainWindow::initUi() {
  SetIcons((const wxIconBundle &)*wxXmlResource::Get()
             ->LoadObject(nullptr, "AppIcon", "wxIcon"));
  initMenu();
}

void MainWindow::initMenu() {
  auto menuBar = new wxMenuBar;
  SetMenuBar(menuBar);

  auto mnuFile = new wxMenu;
  menuBar->Append(mnuFile, wxT("文件(&F)"));

  mnuFile->Append(wxID_NEW, wxT("新建(&N)\tCtrl+N"));
  // TODO new file with tools

  mnuFile->Append(wxID_OPEN, wxT("打开(&O)\tCtrl+O"));

  mnuFile->AppendSeparator();

  mnuFile->Append(wxID_SAVE, wxT("保存(&S)\tCtrl+S"));
  // TODO save handler & enabled flag
  // connect(m_actSave, &QAction::triggered, this, &MainWindow::saveFile);
  // connect(&m_loaded, &BoolValue::changed, m_actSave, &QAction::setEnabled);

  mnuFile->Append(wxID_SAVEAS, wxT("另存为..."));
  // TODO save handler & enabled flag
  // connect(m_actSaveAs, &QAction::triggered, this, &MainWindow::saveFileAs);
  // connect(&m_loaded, &BoolValue::changed, m_actSaveAs, &QAction::setEnabled);

  mnuFile->AppendSeparator();

  mnuFile->Append(wxID_EXIT);
  // TODO confirm exit
  Bind(
    wxEVT_MENU,
    [=](wxCommandEvent &) { Close(true); },
    wxID_EXIT);

  auto mnuEdit = new wxMenu;
  menuBar->Append(mnuEdit, wxT("编辑(&E)"));

  mnuEdit->Append(wxID_UNDO, wxT("撤销\tCtrl+Z"));
  mnuEdit->Append(wxID_REDO, wxT("重做\tCtrl+Y"));
  mnuEdit->AppendSeparator();
  mnuEdit->Append(wxID_COPY, wxT("复制\tCtrl+C"));
  mnuEdit->Append(wxID_CUT, wxT("剪切\tCtrl+X"));
  mnuEdit->Append(wxID_PASTE, wxT("粘贴\tCtrl+V"));
  mnuEdit->AppendSeparator();
  mnuEdit->Append(wxID_SELECTALL, wxT("全选\tCtrl+A"));
  mnuEdit->AppendSeparator();
  mnuEdit->Append(wxID_FIND, wxT("查找\tCtrl+F"));
  mnuEdit->Append(wxID_REPLACE, wxT("替换\tCtrl+R"));

  auto mnuProg = new wxMenu;
  menuBar->Append(mnuProg, wxT("程序(&P)"));

  mnuProg->Append(ID_Menu_Start, wxT("运行\tF5"));
  mnuProg->Append(ID_Menu_Stop, wxT("停止\tCtrl+F7"));
  mnuProg->AppendSeparator();

  mnuProg->Append(ID_Menu_ReloadConfig, wxT("重新加载配置文件"));
  // TODO
  // connect(actConfig, &QAction::triggered, this, [this] { loadConfig(this); });

  auto mnuHelp = new wxMenu;
  menuBar->Append(mnuHelp, wxT("帮助"));

  mnuHelp->Append(ID_Menu_CheckVersion, wxT("检查新版本"));
  // TODO check new version handler
  // connect(actCheckVer, &QAction::triggered, this, [this] {
  //   showMessage("正在检查版本更新", 1000, MessageType::Info);
  //   checkNewVersion(true);
  // });

  mnuHelp->AppendSeparator();

  mnuHelp->Append(wxID_HELP);
  Bind(wxEVT_MENU, &MainWindow::onHelp, this, wxID_HELP);

  mnuHelp->Append(wxID_ABOUT);
  Bind(wxEVT_MENU, &MainWindow::onAbout, this, wxID_ABOUT);
}

void MainWindow::onHelp(wxCommandEvent &) {
  wxNotificationMessage notif;
  notif.SetParent(this);
  notif.SetTitle("wtf??? Hello");
  notif.SetMessage("Yes some!!!!!!");
  notif.Show(wxNotificationMessage::Timeout_Auto);
}

void MainWindow::onAbout(wxCommandEvent &) {
  wxAboutDialogInfo aboutInfo;
  aboutInfo.SetName(wxT("文曲星工具箱"));
  auto version = api::version();
  aboutInfo.SetVersion(wxString(version.data, version.len));
  aboutInfo.SetDescription(
    wxT("目前包含 GVBASIC 编辑器/模拟器。\n"
        "GVBASIC 编辑器的图标来源：\n"
        "Noto Emoji: https://github.com/googlefonts/noto-emoji\n"
        "Elementary OS Icons: https://github.com/elementary/icons\n"));
  aboutInfo.SetWebSite(wxT("https://github.com/arucil/wqxtools"));
  aboutInfo.SetLicense(
    wxT("MIT License\n\
\n\
Copyright (c) 2020-2022 plodsoft\n\
\n\
Permission is hereby granted, free of charge, to any person obtaining a copy\n\
of this software and associated documentation files (the \"Software\"), to deal\n\
in the Software without restriction, including without limitation the rights\n\
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell\n\
copies of the Software, and to permit persons to whom the Software is\n\
furnished to do so, subject to the following conditions:\n\
\n\
The above copyright notice and this permission notice shall be included in all\n\
copies or substantial portions of the Software.\n\
\n\
THE SOFTWARE IS PROVIDED \"AS IS\", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR\n\
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,\n\
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE\n\
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER\n\
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,\n\
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE\n\
SOFTWARE."));
  aboutInfo.AddDeveloper(wxT("arucil"));
  wxAboutBox(aboutInfo, this);
}