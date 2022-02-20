TEMPLATE = app
CONFIG += c++17 debug_and_release
TARGET = wqxtools

DEFINES += QT_DISABLE_DEPRECATED_BEFORE=0x050F00 \
    NO_CXX11_REGEX \
    SCINTILLA_QT \
    _CRT_SECURE_NO_DEPRECATE

INCLUDEPATH += \
    include \
    src/scintilla-qt \
    scintilla/src \
    scintilla/include \
    3rdparty

HEADERS += $$files(src/*.h, true)
SOURCES += $$files(src/*.cpp, true) \
    $$files(scintilla/src/*.cpp, true)

QT += widgets network statemachine core5compat help

RESOURCES += resources/wqxtools.qrc

RC_ICONS = resources/images/Icon.ico

LIBS += $$PWD/scintilla/bin/libScintillaEdit.a \
    $$PWD/../target/release/libapi_cpp_binding.a

unix {
    LIBS += -ldl
}
win32 {
}

CONFIG(debug, debug|release) {
    DESTDIR = build/debug
}
CONFIG(release, debug|release) {
    DESTDIR = build/release
    DEFINES += NDEBUG
}

PRECOMPILED_DIR = $$DESTDIR
OBJECTS_DIR = $$DESTDIR/.obj
MOC_DIR = $$DESTDIR/.moc
RCC_DIR = $$DESTDIR/.qrc
UI_DIR = $$DESTDIR/.ui
