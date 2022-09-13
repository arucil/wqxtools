#include "icon_bundle_handler.h"

#include <wx/bitmap.h>
#include <wx/iconbndl.h>
#include <wx/wx.h>
#include <wx/xml/xml.h>

wxIMPLEMENT_DYNAMIC_CLASS(wxIconBundleXmlHandler, wxXmlResourceHandler);

wxIconBundleXmlHandler::wxIconBundleXmlHandler() : wxXmlResourceHandler() {}

wxObject *wxIconBundleXmlHandler::DoCreateResource() {
  auto path = GetFilePath(m_node);
  if (path.empty()) {
    return new wxIconBundle();
  }
  auto file = GetCurFileSystem().OpenFile(path, wxFS_READ | wxFS_SEEKABLE);
  if (file == nullptr) {
    ReportParamError(
      m_node->GetName(),
      wxString::Format("cannot open bitmap resource \"%s\"", path));
    return new wxIconBundle();
    return new wxIconBundle();
  }
  auto bundle = new wxIconBundle(*file->GetStream(), wxBITMAP_TYPE_ICO);
  delete file;
  if (!bundle->IsOk()) {
    ReportParamError(
      m_node->GetName(),
      wxString::Format("cannot create icon from \"%s\"", path));
    delete bundle;
    return new wxIconBundle();
  }
  return bundle;
}

bool wxIconBundleXmlHandler::CanHandle(wxXmlNode *node) {
  return IsOfClass(node, wxT("wxIcon"));
}
