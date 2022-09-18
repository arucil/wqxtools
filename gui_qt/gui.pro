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
    ../3rdparty

HEADERS += $$files(src/*.h, true)
SOURCES += $$files(src/*.cpp, true) \
    $$files(scintilla/src/*.cxx, true)

QT += widgets network help

RESOURCES += resources/wqxtools.qrc

RC_ICONS = resources/images/Icon.ico


contains(QMAKE_TARGET.arch, x86_64) {
    LIBS += $$PWD/../target/release/libapi_cpp_binding.a
} else {
    LIBS += $$PWD/../target/i686-pc-windows-gnu/release/libapi_cpp_binding.a
}

unix {
    LIBS += -ldl
}
win32 {
    LIBS += -lbcrypt -lwsock32 -lws2_32 -luserenv
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
