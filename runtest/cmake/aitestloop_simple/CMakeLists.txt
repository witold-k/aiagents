cmake_minimum_required(VERSION 4.0)

project(hello-cmake VERSION 1.0 LANGUAGES CXX)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

add_executable(aitestloop
    src/main.cpp
)

enable_testing()

add_test(
    NAME main
    COMMAND aitestloop
)
