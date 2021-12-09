######################################################################
# Automatically generated by qmake (3.1) Tue Oct 19 11:16:52 2021
######################################################################

TEMPLATE = app
CONFIG += c++17
TARGET = wqxtools

INCLUDEPATH += \
    include \
    scintilla/qt/ScintillaEdit \
    scintilla/qt/ScintillaEditBase \
    scintilla/src \
    scintilla/include

HEADERS += src/mainwindow.h \
    src/capability.h \
    src/value.h \
    src/action.h \
    src/config.h \
    src/tool.h \
    src/about_dialog.h \
    src/gvb/gvbeditor.h \
    src/gvb/gvbsim_window.h \
    src/gvb/gvbsim_screen.h \
    src/gvb/gvbsim_keyboard.h \
    src/gvb/gvbsim_input_dialog.h \
    src/gvb/binding_model.h \
    src/gvb/binding_delegate.h \
    src/gvb/array_dialog.h \
    src/gvb/array_model.h \
    src/gvb/editor_model.h
SOURCES += src/*.cpp \
    src/gvb/*.cpp

QT += widgets

RESOURCES += wqxtools.qrc

LIBS += $$PWD/scintilla/bin/libScintillaEdit.a \
    $$PWD/../target/release/libapi_cpp_binding.a \
    -lpthread \
    -ldl


release: DESTDIR = build/release
debug:   DESTDIR = build/debug

OBJECTS_DIR = $$DESTDIR/.obj
MOC_DIR = $$DESTDIR/.moc
RCC_DIR = $$DESTDIR/.qrc
UI_DIR = $$DESTDIR/.ui