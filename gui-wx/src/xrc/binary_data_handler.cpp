#include "binary_data_handler.h"

#include <wx/bitmap.h>
#include <wx/iconbndl.h>
#include <wx/wx.h>
#include <wx/xml/xml.h>

#include "../binary_data.h"
#include "../utils.h"

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
  auto buffer = readAll(*file->GetStream());
  delete file;
  if (!buffer.has_value()) {
    ReportParamError(
      m_node->GetName(),
      wxString::Format("cannot create binary data from \"%s\"", path));
    return new BinaryData;
  }
  return new BinaryData(buffer.value());
}

bool BinaryDataXmlHandler::CanHandle(wxXmlNode *node) {
  return IsOfClass(node, wxT("data"));
}
