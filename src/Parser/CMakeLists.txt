# Prefer Homebrew Flex if available.
set(CMAKE_PROGRAM_PATH ${CMAKE_PROGRAM_PATH} /usr/local/opt/flex/bin)
include_directories(/usr/local/opt/flex/include)

find_package(FLEX 2.6 REQUIRED)
flex_target(LEXER Lexer.l ${CMAKE_CURRENT_SOURCE_DIR}/Lexer.cpp)

add_library(Parser STATIC
  Parser.cpp
  ${FLEX_LEXER_OUTPUTS})
