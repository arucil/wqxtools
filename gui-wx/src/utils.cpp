#include "utils.h"

std::optional<wxMemoryBuffer> readAll(wxInputStream &input) {
  auto len = input.GetLength();
  if (len == wxInvalidOffset) {
    wxMemoryBuffer buffer;
#define BUF_SIZE 1024
    char *buf = new char[BUF_SIZE];
    while (input.Read(buf, BUF_SIZE).LastRead() == BUF_SIZE) {
      buffer.AppendData(buf, input.LastRead());
    }
    delete[] buf;
    switch (input.GetLastError()) {
      case wxSTREAM_EOF:
      case wxSTREAM_NO_ERROR:
        return buffer;
      case wxSTREAM_READ_ERROR:
      case wxSTREAM_WRITE_ERROR:
        return {};
    }
  } else {
    char *data = new char[len];
    if (!input.ReadAll(data, len)) {
      delete[] data;
      return {};
    }
    wxMemoryBuffer buffer(len);
    buffer.AppendData(data, len);
    delete[] data;
    return buffer;
  }
  return {};
}

void showErrorMessage(const wxString &message, wxWindow *parent) {
  wxMessageBox(message, wxT("错误"), wxICON_ERROR, parent);
}