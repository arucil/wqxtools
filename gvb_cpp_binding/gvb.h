#ifndef GVB_H
#define GVB_H

#include <cstdint>
using std::size_t;


namespace gvb {

struct Document;

struct CString {
  const char *data;
  size_t len;
};

struct LoadedDocument {
  Document *doc;
  CString text;
};

extern "C" {

LoadedDocument load_document(const unsigned short *path, size_t len);

void destroy_document(Document *doc);

void destroy_string(CString str);

} // extern "C"

} // namespace gvb

#endif // GVB_H
