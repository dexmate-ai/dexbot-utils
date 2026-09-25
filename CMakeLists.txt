cmake_minimum_required(VERSION 3.20)
project(dexbot VERSION 0.2.3 LANGUAGES C CXX)
include(GNUInstallDirs)
include(CMakePackageConfigHelpers)
find_program(CARGO_EXECUTABLE cargo REQUIRED)
if(NOT UNIX)
  message(FATAL_ERROR "The native SDK currently supports Linux and macOS")
endif()
set(DEXBOT_CARGO_TARGET_DIR "${CMAKE_CURRENT_BINARY_DIR}/cargo")
set(DEXBOT_NATIVE_LIBRARY "${DEXBOT_CARGO_TARGET_DIR}/release/${CMAKE_SHARED_LIBRARY_PREFIX}dexbot_model${CMAKE_SHARED_LIBRARY_SUFFIX}")
file(GLOB_RECURSE DEXBOT_RUST_INPUTS CONFIGURE_DEPENDS "${CMAKE_CURRENT_SOURCE_DIR}/crates/*")
add_custom_command(OUTPUT "${DEXBOT_NATIVE_LIBRARY}" "${DEXBOT_CARGO_TARGET_DIR}/release/dexbot"
  COMMAND "${CMAKE_COMMAND}" -E env "CARGO_TARGET_DIR=${DEXBOT_CARGO_TARGET_DIR}" "${CARGO_EXECUTABLE}"
    build --release --locked --manifest-path "${CMAKE_CURRENT_SOURCE_DIR}/Cargo.toml" -p dexbot-capi -p dexbot-cli
  DEPENDS ${DEXBOT_RUST_INPUTS} Cargo.toml Cargo.lock
  WORKING_DIRECTORY "${CMAKE_CURRENT_SOURCE_DIR}" VERBATIM)
add_custom_target(dexbot_native ALL DEPENDS "${DEXBOT_NATIVE_LIBRARY}" "${DEXBOT_CARGO_TARGET_DIR}/release/dexbot")
add_library(dexbot::model SHARED IMPORTED GLOBAL)
set_target_properties(dexbot::model PROPERTIES IMPORTED_LOCATION "${DEXBOT_NATIVE_LIBRARY}"
  INTERFACE_INCLUDE_DIRECTORIES "${CMAKE_CURRENT_SOURCE_DIR}/bindings/c/include;${CMAKE_CURRENT_SOURCE_DIR}/bindings/cpp/include"
  INTERFACE_COMPILE_FEATURES cxx_std_17)
add_dependencies(dexbot::model dexbot_native)
install(FILES "${DEXBOT_NATIVE_LIBRARY}" DESTINATION "${CMAKE_INSTALL_LIBDIR}")
install(PROGRAMS "${DEXBOT_CARGO_TARGET_DIR}/release/dexbot" DESTINATION "${CMAKE_INSTALL_BINDIR}")
install(DIRECTORY bindings/c/include/ bindings/cpp/include/ DESTINATION "${CMAKE_INSTALL_INCLUDEDIR}")
install(FILES LICENSE README.md DESTINATION "${CMAKE_INSTALL_DATADIR}/dexbot")
install(DIRECTORY examples/cpp/ DESTINATION "${CMAKE_INSTALL_DATADIR}/dexbot/examples/cpp")
configure_package_config_file(cmake/dexbotConfig.cmake.in "${CMAKE_CURRENT_BINARY_DIR}/dexbotConfig.cmake"
  INSTALL_DESTINATION "${CMAKE_INSTALL_LIBDIR}/cmake/dexbot" PATH_VARS CMAKE_INSTALL_LIBDIR CMAKE_INSTALL_INCLUDEDIR)
write_basic_package_version_file("${CMAKE_CURRENT_BINARY_DIR}/dexbotConfigVersion.cmake" VERSION ${PROJECT_VERSION} COMPATIBILITY SameMinorVersion)
install(FILES "${CMAKE_CURRENT_BINARY_DIR}/dexbotConfig.cmake" "${CMAKE_CURRENT_BINARY_DIR}/dexbotConfigVersion.cmake"
  DESTINATION "${CMAKE_INSTALL_LIBDIR}/cmake/dexbot")

install(FILES crates/dexbot-model/assets/urdf/LICENSE
  DESTINATION "${CMAKE_INSTALL_DATADIR}/dexbot" RENAME LICENSE-URDF)
