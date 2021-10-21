#include <ILexer.h>

#define SCE_C_COMMENT 1
#define SCE_C_COMMENTLINE 2
#define SCE_C_COMMENTDOC 3
#define SCE_C_NUMBER 4
#define SCE_C_WORD 5
#define SCE_C_STRING 6
#define SCE_C_CHARACTER 7
#define SCE_C_UUID 8
#define SCE_C_PREPROCESSOR 9
#define SCE_C_OPERATOR 10
#define SCE_C_IDENTIFIER 11
#define SCE_C_STRINGEOL 12
#define SCE_C_VERBATIM 13
#define SCE_C_REGEX 14
#define SCE_C_COMMENTLINEDOC 15
#define SCE_C_WORD2 16
#define SCE_C_COMMENTDOCKEYWORD 17
#define SCE_C_COMMENTDOCKEYWORDERROR 18
#define SCE_C_GLOBALCLASS 19
#define SCE_C_STRINGRAW 20
#define SCE_C_TRIPLEVERBATIM 21
#define SCE_C_HASHQUOTEDSTRING 22
#define SCE_C_PREPROCESSORCOMMENT 23
#define SCE_C_PREPROCESSORCOMMENTDOC 24
#define SCE_C_USERLITERAL 25
#define SCE_C_TASKMARKER 26
#define SCE_C_ESCAPESEQUENCE 27

class GvbLexer : public Scintilla::ILexer5 {
public:
  GvbLexer();
  ~GvbLexer();

  int SCI_METHOD Version() const override;
  void SCI_METHOD Release() override;
  const char *SCI_METHOD PropertyNames() override;
  int SCI_METHOD PropertyType(const char *name) override;
  const char *SCI_METHOD DescribeProperty(const char *name) override;
  Sci_Position SCI_METHOD PropertySet(const char *key,
                                      const char *val) override;
  const char *SCI_METHOD DescribeWordListSets() override;
  Sci_Position SCI_METHOD WordListSet(int n, const char *wl) override;
  void SCI_METHOD Lex(Sci_PositionU startPos, Sci_Position lengthDoc,
                      int initStyle, Scintilla::IDocument *pAccess) override;
  void SCI_METHOD Fold(Sci_PositionU startPos, Sci_Position lengthDoc,
                       int initStyle, Scintilla::IDocument *pAccess) override;
  void *SCI_METHOD PrivateCall(int operation, void *pointer) override;
  int SCI_METHOD LineEndTypesSupported() override;
  int SCI_METHOD AllocateSubStyles(int styleBase, int numberStyles) override;
  int SCI_METHOD SubStylesStart(int styleBase) override;
  int SCI_METHOD SubStylesLength(int styleBase) override;
  int SCI_METHOD StyleFromSubStyle(int subStyle) override;
  int SCI_METHOD PrimaryStyleFromStyle(int style) override;
  void SCI_METHOD FreeSubStyles() override;
  void SCI_METHOD SetIdentifiers(int style, const char *identifiers) override;
  int SCI_METHOD DistanceToSecondaryStyles() override;
  const char *SCI_METHOD GetSubStyleBases() override;
  int SCI_METHOD NamedStyles() override;
  const char *SCI_METHOD NameOfStyle(int style) override;
  const char *SCI_METHOD TagsOfStyle(int style) override;
  const char *SCI_METHOD DescriptionOfStyle(int style) override;

  const char *SCI_METHOD GetName() override;
  int SCI_METHOD GetIdentifier() override;
  const char *SCI_METHOD PropertyGet(const char *key) override;
};