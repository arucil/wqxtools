######################################################################
# Automatically generated by qmake (3.1) Tue Oct 19 11:16:52 2021
######################################################################

TEMPLATE = app
CONFIG += c++17 debug_and_release
TARGET = wqxtools

INCLUDEPATH += \
    include \
    scintilla/qt/ScintillaEdit \
    scintilla/qt/ScintillaEditBase \
    scintilla/src \
    scintilla/include \
    3rdparty

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
    src/gvb/table_editor_delegate.h \
    src/gvb/array_edit_dialog.h \
    src/gvb/array_model.h \
    src/gvb/table_editor_model.h \
    src/gvb/code_editor.h \
    src/gvb/double_spinbox.h \
    src/gvb/search_bar.h

SOURCES += src/*.cpp \
    src/gvb/*.cpp

QT += widgets

RESOURCES += resources/wqxtools.qrc

LIBS += $$PWD/scintilla/bin/libScintillaEdit.a \
    $$PWD/../target/release/libapi_cpp_binding.a \
    -ldl

CONFIG(debug, debug|release) {
    DESTDIR = build/debug
}
CONFIG(release, debug|release) {
    DESTDIR = build/release
}

PRECOMPILED_DIR = $$DESTDIR
OBJECTS_DIR = $$DESTDIR/.obj
MOC_DIR = $$DESTDIR/.moc
RCC_DIR = $$DESTDIR/.qrc
UI_DIR = $$DESTDIR/.ui
