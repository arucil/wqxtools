#include "binary_data_handler.h"

#include <wx/bitmap.h>
#include <wx/iconbndl.h>
#include <wx/wx.h>
#include <wx/xml/xml.h>

#include "../binary_data.h"

wxIMPLEMENT_DYNAMIC_CLASS(BinaryDataXmlHandler, wxXmlResourceHandler);

BinaryDataXmlHandler::BinaryDataXmlHandler() : wxXmlResourceHandler() {}

wxObject *BinaryDataXmlHandler::DoCreateResource() {
  auto path = GetFilePath(m_node);
  if (path.empty()) {
    return new BinaryData;
  }
  auto file = GetCurFileSystem().OpenFile(path, wxFS_READ | wxFS_SEEKABLE);
  if (file == nullptr) {
    ReportParamError(
      m_node->GetName(),
      wxString::Format("cannot open binary data resource \"%s\"", path));
    return new BinaryData;
  }
  auto len = file->GetStream()->GetSize();
  void *data = new char[len];
  if (!file->GetStream()->ReadAll(data, len)) {
    delete[] static_cast<char *>(data);
    delete file;
    ReportParamError(
      m_node->GetName(),
      wxString::Format("cannot create binary data from \"%s\"", path));
    return new BinaryData;
  }
  delete file;
  auto bundle = new BinaryData(data, len);
  delete[] static_cast<char *>(data);
  return bundle;
}

bool BinaryDataXmlHandler::CanHandle(wxXmlNode *node) {
  return IsOfClass(node, wxT("data"));
}
