#pragma once

#include <wx/wx.h>

class BinaryData: public wxObject {
public:
  BinaryData() {}
  BinaryData(void *data, size_t len) : m_buffer(len) {
    m_buffer.AppendData(data, len);
  }
  virtual ~BinaryData() {}

  const wxMemoryBuffer &Buffer() const {
    return m_buffer;
  };

private:
  wxMemoryBuffer m_buffer;

  wxDECLARE_DYNAMIC_CLASS(BinaryData);
};