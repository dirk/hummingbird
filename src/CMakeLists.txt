add_subdirectory(Parser)

add_executable(hummingbird main.cpp)
target_link_libraries(hummingbird Parser)
