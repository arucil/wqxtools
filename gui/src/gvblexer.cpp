#include "gvblexer.h"
#include <Scintilla.h>

#define SCLEX_AUTOMATIC 1000

GvbLexer::GvbLexer() {}

GvbLexer::~GvbLexer() {}

void SCI_METHOD GvbLexer::Release() { delete this; }

int SCI_METHOD GvbLexer::Version() const { return Scintilla::lvRelease5; }

const char *SCI_METHOD GvbLexer::PropertyNames() { return ""; }

int SCI_METHOD GvbLexer::PropertyType(const char *) { return SC_TYPE_BOOLEAN; }

const char *SCI_METHOD GvbLexer::DescribeProperty(const char *) { return ""; }

Sci_Position SCI_METHOD
GvbLexer::PropertySet(const char *key, const char *val) {}

const char *SCI_METHOD GvbLexer::PropertyGet(const char *key) {
  return nullptr;
}

const char *SCI_METHOD GvbLexer::DescribeWordListSets() { return ""; }

Sci_Position SCI_METHOD GvbLexer::WordListSet(int n, const char *wl) {
  /* TODO
if (n < numWordLists) {
  if (keyWordLists[n]->Set(wl)) {
    return 0;
  }
}
return -1;
*/
  return 0;
}

void *SCI_METHOD GvbLexer::PrivateCall(int, void *) { return nullptr; }

int SCI_METHOD GvbLexer::LineEndTypesSupported() {
  return SC_LINE_END_TYPE_DEFAULT;
}

int SCI_METHOD GvbLexer::AllocateSubStyles(int, int) { return -1; }

int SCI_METHOD GvbLexer::SubStylesStart(int) { return -1; }

int SCI_METHOD GvbLexer::SubStylesLength(int) { return 0; }

int SCI_METHOD GvbLexer::StyleFromSubStyle(int subStyle) { return subStyle; }

int SCI_METHOD GvbLexer::PrimaryStyleFromStyle(int style) { return style; }

void SCI_METHOD GvbLexer::FreeSubStyles() {}

void SCI_METHOD GvbLexer::SetIdentifiers(int, const char *) {}

int SCI_METHOD GvbLexer::DistanceToSecondaryStyles() { return 0; }

static const char styleSubable[] = {0};

const char *SCI_METHOD GvbLexer::GetSubStyleBases() { return styleSubable; }

int SCI_METHOD GvbLexer::NamedStyles() { return 0; }

const char *SCI_METHOD GvbLexer::NameOfStyle(int style) { return ""; }

const char *SCI_METHOD GvbLexer::TagsOfStyle(int style) { return ""; }

const char *SCI_METHOD GvbLexer::DescriptionOfStyle(int style) { return ""; }

const char *SCI_METHOD GvbLexer::GetName() { return ""; }

int SCI_METHOD GvbLexer::GetIdentifier() { return SCLEX_AUTOMATIC; }

void SCI_METHOD GvbLexer::Lex(
    Sci_PositionU startPos, Sci_Position lengthDoc, int initStyle,
    Scintilla::IDocument *pAccess) {
  /* TODO
Accessor astyler(pAccess, &props);
module->Lex(startPos, lengthDoc, initStyle, keyWordLists, astyler);
astyler.Flush();
*/
}

void SCI_METHOD GvbLexer::Fold(
    Sci_PositionU startPos, Sci_Position lengthDoc, int initStyle,
    Scintilla::IDocument *pAccess) {}
