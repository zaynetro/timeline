# Development setup

## Requirements

* Flutter
* Rust nightly


## Icons

* Icons are generated using https://pub.dev/packages/flutter_launcher_icons
* Configuration is at the bottom of `app/pubspec.yaml`

```
cd app
flutter pub get
flutter pub run flutter_launcher_icons
```


## Generate Flutter Rust bridge bindings

From `app` directory.

```
flutter_rust_bridge_codegen \
    -r native/src/api.rs \
    -d lib/bridge_generated.dart \
    -c ios/Runner/bridge_generated.h \
    -e macos/Runner/
```

On Fedora Silverblue:

```fish
set -x CPATH "$(clang -v 2>&1 | grep "Selected GCC installation" | rev | cut -d' ' -f1 | rev)/include"

flutter_rust_bridge_codegen --llvm-path /usr/lib64/libclang.so.14.0.0 \
    -r native/src/api.rs \
    -d lib/bridge_generated.dart \
    -c ios/Runner/bridge_generated.h \
    -e macos/Runner/
```


## iOS

```
cd app
flutter create --platforms=ios .
cargo install -f cargo-xcode
cargo install -f flutter_rust_bridge_codegen@1.59.0
rustup target add aarch64-apple-ios
```

* Follow Flutter Rust bridge setup for iOS[^frb-ios]
* Modify how Xcode strips symbols[^ios-symbols]
    * In Xcode, go to **Target Runner > Build Settings > Strip Style**.
    * Change from **All Symbols** to **Non-Global Symbols**.


[^frb-ios]: https://cjycode.com/flutter_rust_bridge/integrate/ios.html
[^ios-symbols]: https://docs.flutter.dev/development/platform-integration/ios/c-interop#stripping-ios-symbols


## Mac

```
cd app
flutter create --platforms=macos .
cargo install -f cargo-xcode
cargo install -f flutter_rust_bridge_codegen@1.59.0
```

* Follow Flutter Rust bridge setup for Mac[^frb-ios]
    * Instead of dylib as suggested in the guide I am linking static lib (just like on iOS)
* Under **Signing & Capabilities**
    * Check "Outgoing connections" for both Debug and Release profiles
* Modify how Xcode strips symbols[^ios-symbols]
    * In Xcode, go to **Target Runner > Build Settings > Strip Style**.
    * Change from **All Symbols** to **Non-Global Symbols**.


## Android

```
cd app
flutter create --platforms=android .
cargo install -f cargo-ndk
cargo install -f flutter_rust_bridge_codegen@1.59.0
rustup target add \
    aarch64-linux-android \
    armv7-linux-androideabi \
    x86_64-linux-android
```

* In Android Studio also install:
    * Android SDK Command-line Tools
    * NDK

* Follow Flutter Rust bridge setup for Android[^frb-android]
    * Or see their sample project[^frb-template]
    * Add to `~/.gradle/gradle.properties` (using absolute path is a requirement):
        `ANDROID_NDK=/var/home/roman/Android/Sdk/ndk/25.0.8775105`
        Mac: `ANDROID_NDK=/Users/roman/Library/Android/sdk/ndk/25.1.8937393`

### Running on physical Android device

You have two options:

1. Either clear debug signing config not to use release keystore
2. Or set up keystore (see "Android: First time setup" section in `RELEASE.md`)

[^frb-android]: https://cjycode.com/flutter_rust_bridge/integrate/android_tasks.html
[^frb-template]: https://github.com/Desdaemon/flutter_rust_bridge_template/blob/main/android/app/build.gradle
