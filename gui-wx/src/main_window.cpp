#include "main_window.h"

#include <wx/aboutdlg.h>
#include <wx/html/helpctrl.h>
#include <wx/html/helpwnd.h>
#include <wx/webrequest.h>
#include <wx/xrc/xmlres.h>

#include <optional>

#include "api.h"
#include "new_version_dialog.h"
#include "nlohmann/json.hpp"
#include "notification.h"
#include "utils.h"

#if wxUSE_WEBREQUEST_CURL
  #include <curl/curl.h>
#elif defined(__WXMSW__)
  #include <winhttp.h>
#else
  #error "this platform is not supported"
#endif

#define WINDOW_TITLE "文曲星工具箱"
#define UNNAMED "未命名"
#define STYLE_DIR "styles"

#define VERSION_API_ENDPOINT \
  "https://gitlab.com/api/v4/projects/32814745/releases"

using json = nlohmann::json;

enum {
  ID_Menu_CheckVersion,
};

MainWindow::MainWindow(const wxString &) :
  wxFrame(nullptr, wxID_ANY, wxT(""), wxDefaultPosition, wxSize(400, 340)) {
  auto &web = wxWebSession::GetDefault();
#if wxUSE_WEBREQUEST_CURL
  auto handle = reinterpret_cast<CURLM *>(web.GetNativeHandle());
  curl_easy_setopt(handle, CURLOPT_TIMEOUT, 3);
#elif defined(__MXMSW__)
  auto handle = reinterpret_cast<HINTERNET>(web.GetNativeHandle());
  WinHttpSetTimeouts(handle, 1000, 1000, 1000, 1000);
#endif

  // TODO drag'n'drop
  initUi();

  Center();

  CallAfter([&] {
    setFileLoaded(false);

    auto menuBar = GetMenuBar();
    menuBar->Enable(wxID_SAVE, false);
    menuBar->Enable(wxID_SAVEAS, false);
    menuBar->Enable(wxID_UNDO, false);
    menuBar->Enable(wxID_REDO, false);
    menuBar->Enable(wxID_COPY, false);
    menuBar->Enable(wxID_CUT, false);
    menuBar->Enable(wxID_PASTE, false);
    menuBar->Enable(wxID_FIND, false);
    menuBar->Enable(wxID_REPLACE, false);
    menuBar->Enable(wxID_EXECUTE, false);
    menuBar->Enable(wxID_STOP, false);

    checkNewVersion(false);
  });
}

void MainWindow::initUi() {
  auto icons = wxXmlResource::Get()->LoadObject(nullptr, "AppIcon", "wxIcon");
  SetIcons((const wxIconBundle &)*icons);
  delete icons;
  initMenu();

  auto panel = new wxPanel(this, wxID_ANY);
  auto panelSizer = new wxBoxSizer(wxHORIZONTAL);
  panel->SetSizer(panelSizer);
  auto box = new wxStaticBox(panel, wxID_ANY, wxString());
  panelSizer->Add(box, 1, wxEXPAND | wxALL, 20);
  auto boxSizer = new wxBoxSizer(wxHORIZONTAL);
  box->SetSizer(boxSizer);
  auto label = new wxStaticText(
    box,
    wxID_ANY,
    wxT("点击菜单 [文件] -> [打开] 打开文件\n"
        "或拖动文件到此窗口"),
    wxDefaultPosition,
    wxDefaultSize,
    wxALIGN_CENTRE_HORIZONTAL | wxST_NO_AUTORESIZE);
  boxSizer->Add(label, 1, wxALIGN_CENTER_VERTICAL);

  updateTitle();
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

  mnuFile->Append(wxID_SAVEAS, wxT("另存为..."));
  // TODO save handler & enabled flag
  // connect(m_actSaveAs, &QAction::triggered, this, &MainWindow::saveFileAs);

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

  mnuProg->Append(wxID_EXECUTE, wxT("运行\tF5"));
  mnuProg->Append(wxID_STOP, wxT("停止\tCtrl+F7"));
  mnuProg->AppendSeparator();

  mnuProg->Append(wxID_REFRESH, wxT("重新加载配置文件"));
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
  if (!m_helpCtrl.has_value()) {
    m_helpCtrl.emplace();
    m_helpCtrl.value().AddBook(wxT("memory:help.zip"));
  }
  m_helpCtrl.value().DisplayContents();
}

void MainWindow::onAbout(wxCommandEvent &) {
  wxAboutDialogInfo aboutInfo;
  aboutInfo.SetName(wxT("文曲星工具箱"));
  auto version = api::version();
  aboutInfo.SetVersion(wxString(version.data, version.len));
  aboutInfo.SetDescription(
    wxT("目前包含 GVBASIC 编辑器/模拟器。\n"
        "\n"
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

void MainWindow::setFileLoaded(bool loaded) {
  auto menuBar = GetMenuBar();
  menuBar->Enable(wxID_SAVE, loaded);
  menuBar->Enable(wxID_SAVEAS, loaded);
}

void MainWindow::setOpenFile(const wxString &path) {
  m_openFilePath = path;
  updateTitle();
}

void MainWindow::updateTitle() {
  // TODO
}

void MainWindow::checkNewVersion(bool isManual) {
  auto request =
    wxWebSession::GetDefault().CreateRequest(this, VERSION_API_ENDPOINT);

  if (!request.IsOk()) {
    if (isManual) {
      showErrorMessage(wxT("检查版本失败：无法初始化网络请求"), this);
    }
    return;
  }

  Bind(wxEVT_WEBREQUEST_STATE, [=](wxWebRequestEvent &evt) {
    switch (evt.GetState()) {
      // Request completed
      case wxWebRequest::State_Completed: {
        auto &resp = evt.GetResponse();
        if (resp.GetStatus() != 200) {
          if (isManual) {
            showErrorMessage(
              wxString::Format(
                wxT("获取新版本信息失败：Gitlab 响应码 %d"),
                resp.GetStatus()),
              this);
          }
          return;
        }

        auto buffer = readAll(*evt.GetResponse().GetStream());
        if (!buffer.has_value()) {
          if (isManual) {
            showErrorMessage(wxT("获取新版本信息失败：无法读取版本信息"), this);
          }
          return;
        }

        auto start = static_cast<const char *>(buffer.value().GetData());
        json doc;
        try {
          doc = json::parse(start, start + buffer.value().GetDataLen());
        } catch (json::parse_error &e) {
          if (isManual) {
            showErrorMessage(
              wxString::Format(
                wxT("获取新版本信息失败：JSON parse error: %s"),
                wxString::FromUTF8(e.what())),
              this);
          }
          return;
        }

        auto tag = doc[0]["tag_name"].get<json::string_t>();
        auto result =
          api::is_new_version({tag.data(), static_cast<size_t>(tag.size())});
        if (result.tag == api::Maybe<bool>::Tag::Nothing) {
          if (isManual) {
            showErrorMessage(
              wxT("检查版本失败：release tag_name is not semver"));
          }
          return;
        }

        if (isManual) {
          if (result.just._0) {
            notifyNewVersion(tag);
          } else {
            showNotification(
              wxT("已经是最新版本"),
              NotificationType::Information,
              1);
          }
        } else if (result.just._0) {
          showNotification(
            wxT("有新版本，请点击菜单 [帮助] -> [检查新版本] 查看新版本"),
            NotificationType::Information,
            2);
        }

        break;
      }
      // Request failed
      case wxWebRequest::State_Failed:
        if (isManual) {
          showErrorMessage(wxString::Format(
            wxT("获取新版本失败：网络错误：%s"),
            evt.GetErrorDescription()));
          return;
        }
        break;
      default:
        break;
    }
  });

  request.Start();
}

void MainWindow::notifyNewVersion(const wxString &tag) {
  auto request = wxWebSession::GetDefault().CreateRequest(
    this,
    VERSION_API_ENDPOINT "/%1?include_html_description=true");

  if (!request.IsOk()) {
    showErrorMessage(wxT("检查版本失败：无法初始化网络请求"), this);
    return;
  }

  Bind(wxEVT_WEBREQUEST_STATE, [=](wxWebRequestEvent &evt) {
    switch (evt.GetState()) {
      // Request completed
      case wxWebRequest::State_Completed: {
        hideNotification();

        auto &resp = evt.GetResponse();
        if (resp.GetStatus() != 200) {
          showErrorMessage(
            wxString::Format(
              wxT("获取新版本信息失败：Gitlab 响应码 %d"),
              resp.GetStatus()),
            this);
          return;
        }

        auto buffer = readAll(*evt.GetResponse().GetStream());
        if (!buffer.has_value()) {
          showErrorMessage(wxT("获取新版本信息失败：无法读取版本信息"), this);
          return;
        }

        auto start = static_cast<const char *>(buffer.value().GetData());
        json doc;
        try {
          doc = json::parse(start, start + buffer.value().GetDataLen());
        } catch (json::parse_error &e) {
          showErrorMessage(
            wxString::Format(
              wxT("获取新版本信息失败：JSON parse error: %s"),
              wxString::FromUTF8(e.what())),
            this);
          return;
        }

        auto description = doc["description_html"].get<json::string_t>();
        auto url = doc["_links"]["self"].get<json::string_t>();

        NewVersionDialog dialog(
          this,
          wxString::FromUTF8(tag),
          wxString::FromUTF8(description),
          wxString::FromUTF8(url));
        dialog.ShowModal();

        break;
      }
      // Request failed
      case wxWebRequest::State_Failed:
        showErrorMessage(wxString::Format(
          wxT("获取新版本失败：网络错误：%s"),
          evt.GetErrorDescription()));
        return;
      default:
        break;
    }
  });

  request.Start();
}