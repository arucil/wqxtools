#include <wx/fs_arc.h>
#include <wx/fs_mem.h>
#include <wx/imagbmp.h>
#include <wx/image.h>
#include <wx/imagpng.h>
#include <wx/wx.h>
#include <wx/xrc/xh_bmp.h>
#include <wx/xrc/xmlres.h>

#include "binary_data.h"
#include "main_window.h"
#include "xrc/binary_data_handler.h"
#include "xrc/icon_bundle_handler.h"

extern void InitXmlResource();

class App: public wxApp {
public:
  virtual bool OnInit() override {
    if (!wxApp::OnInit())
      return false;
    wxImage::AddHandler(new wxPNGHandler);
    wxImage::AddHandler(new wxICOHandler);
    wxFileSystem::AddHandler(new wxMemoryFSHandler);
    wxFileSystem::AddHandler(new wxArchiveFSHandler);
    wxXmlResource::Get()->AddHandler(new IconBundleXmlHandler);
    wxXmlResource::Get()->AddHandler(new BinaryDataXmlHandler);

    InitXmlResource();

    auto helpData = wxDynamicCast(
      wxXmlResource::Get()->LoadObject(nullptr, "Help", "data"),
      BinaryData);
    wxMemoryFSHandler::AddFileWithMimeType(
      "help.zip",
      helpData->Buffer().GetData(),
      helpData->Buffer().GetDataLen(),
      wxT("application/zip"));
    delete helpData;

    MainWindow *window;
    if (argc > 2) {
      wxMessageBox(wxT("运行参数过多"), wxT("错误"), wxICON_ERROR);
    } else if (argc == 2) {
      window = new MainWindow(argv[0]);
    } else {
      window = new MainWindow;
    }
    window->Show();
    return true;
  }
};

wxIMPLEMENT_APP(App);