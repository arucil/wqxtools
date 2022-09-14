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
    wxMemoryFSHandler::AddFile(
      "help.zip",
      helpData->Buffer().GetData(),
      helpData->Buffer().GetDataLen());
    delete helpData;

    auto window = new MainWindow;
    window->Show();
    return true;
  }
};

wxIMPLEMENT_APP(App);