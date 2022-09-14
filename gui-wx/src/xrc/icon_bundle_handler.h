#pragma once

#include <wx/wx.h>
#include <wx/xrc/xmlres.h>

class WXDLLIMPEXP_XRC IconBundleXmlHandler: public wxXmlResourceHandler {
  wxDECLARE_DYNAMIC_CLASS(IconBundleXmlHandler);

public:
  IconBundleXmlHandler();
  virtual wxObject *DoCreateResource() wxOVERRIDE;
  virtual bool CanHandle(wxXmlNode *node) wxOVERRIDE;
};