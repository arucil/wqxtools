cmake_minimum_required(VERSION 3.2)

project(gui-wx)

set(CMAKE_CXX_STANDARD 17)

file(GLOB_RECURSE SOURCES src/*.cpp)

add_executable(wqxtools ${SOURCES} src/resources.cpp)

if(MSVC)
  target_compile_options(wqxtools PRIVATE /W4 /WX)
else()
  target_compile_options(wqxtools PRIVATE -Wall -Wextra -Wpedantic -Werror)
endif()

find_package(wxWidgets REQUIRED core base net html xrc stc)
include(${wxWidgets_USE_FILE})
include_directories(${CMAKE_SOURCE_DIR}/include)
# and for each of your dependent executable/library targets:
target_link_libraries(wqxtools ${wxWidgets_LIBRARIES})

if(CMAKE_SIZEOF_VOID_P EQUAL 8)
  # 64 bits
  target_link_libraries(wqxtools ${CMAKE_SOURCE_DIR}/../target/release/libapi_cpp_binding.a)
elseif(CMAKE_SIZEOF_VOID_P EQUAL 4)
  # 32 bits
endif()

add_custom_command(
  OUTPUT src/resources.cpp
  COMMAND wxrc resources/wqxtools.xrc -co src/resources.cpp
  DEPENDS resources/wqxtools.xrc resources/help.zip
  WORKING_DIRECTORY ${CMAKE_CURRENT_BINARY_DIR}
)