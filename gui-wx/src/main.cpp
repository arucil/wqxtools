#include <wx/image.h>
#include <wx/imagpng.h>
#include <wx/imagbmp.h>
#include <wx/wx.h>
#include <wx/xrc/xh_bmp.h>
#include <wx/xrc/xmlres.h>

#include "main_window.h"
#include "icon_bundle_handler.h"

extern void InitXmlResource();

class App: public wxApp {
public:
  virtual bool OnInit() override {
    wxImage::AddHandler(new wxPNGHandler);
    wxImage::AddHandler(new wxICOHandler);
    wxXmlResource::Get()->AddHandler(new wxIconBundleXmlHandler);

    InitXmlResource();

    auto window = new MainWindow;
    window->Show();
    return true;
  }
};

wxIMPLEMENT_APP(App);