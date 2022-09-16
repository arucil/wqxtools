#pragma once

#include <wx/wx.h>
#include <utility>

class BinaryData: public wxObject {
public:
  BinaryData() {}
  BinaryData(wxMemoryBuffer buf) : m_buffer(std::move(buf)) {}
  virtual ~BinaryData() {}

  const wxMemoryBuffer &Buffer() const {
    return m_buffer;
  };

private:
  wxMemoryBuffer m_buffer;

  wxDECLARE_DYNAMIC_CLASS(BinaryData);
};