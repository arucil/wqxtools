#pragma once

#include <wx/wx.h>
#include <wx/xrc/xmlres.h>

class WXDLLIMPEXP_XRC wxIconBundleXmlHandler: public wxXmlResourceHandler {
  wxDECLARE_DYNAMIC_CLASS(wxIconBundleXmlHandler);

public:
  wxIconBundleXmlHandler();
  virtual wxObject *DoCreateResource() wxOVERRIDE;
  virtual bool CanHandle(wxXmlNode *node) wxOVERRIDE;
};