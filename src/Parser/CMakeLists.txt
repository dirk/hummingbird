# Prefer Homebrew Flex if available.
set(CMAKE_PROGRAM_PATH ${CMAKE_PROGRAM_PATH} /usr/local/opt/flex/bin)
include_directories(/usr/local/opt/flex/include)

find_package(FLEX 2.6 REQUIRED)
flex_target(LEXER Lexer.l ${CMAKE_CURRENT_SOURCE_DIR}/Lexer.cpp)

# Prefer Homebrew Bison if available.
set(CMAKE_PROGRAM_PATH ${CMAKE_PROGRAM_PATH} /usr/local/opt/bison/bin)
include_directories(/usr/local/opt/bison/include)

find_package(BISON 3.2 REQUIRED)
bison_target(PARSER Parser.yy ${CMAKE_CURRENT_SOURCE_DIR}/Parser.cpp
  DEFINES_FILE ${CMAKE_CURRENT_SOURCE_DIR}/Parser.h)

add_flex_bison_dependency(LEXER PARSER)

add_library(Parser STATIC
  Driver.cpp
  Nodes.cpp
  ${FLEX_LEXER_OUTPUTS}
  ${BISON_PARSER_OUTPUTS})
