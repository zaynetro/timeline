# We include Corrosion inline here, but ideally in a project with
# many dependencies we would need to install Corrosion on the system.
# See instructions on https://github.com/AndrewGaspar/corrosion#cmake-install
# Once done, uncomment this line:
# find_package(Corrosion REQUIRED)

# Install Corrosion
include(FetchContent)
FetchContent_Declare(
    Corrosion
    GIT_REPOSITORY https://github.com/AndrewGaspar/corrosion.git
    #GIT_TAG origin/master
    GIT_TAG v0.3.0
)
FetchContent_MakeAvailable(Corrosion)

set(Rust_TOOLCHAIN "nightly-x86_64-unknown-linux-gnu")
corrosion_import_crate(MANIFEST_PATH ../native/Cargo.toml)

# Flutter-specific
set(CRATE_NAME "native")
target_link_libraries(${BINARY_NAME} PRIVATE ${CRATE_NAME})
list(APPEND PLUGIN_BUNDLED_LIBRARIES $<TARGET_FILE:${CRATE_NAME}-shared>)
