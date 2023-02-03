# Release

## Server

Deploying to fly.io is just:

* `make deploy`

### Server: First time setup

* Create a volume: https://fly.io/docs/reference/volumes/
    * `fly volumes create bolik_api_db --region waw --size 3 -a bolik-api`
    * Create 3GB volume in Warsaw
* Mount volume to your app in `fly-bolik-api.toml` under `[mounts]` section.
* Set secrets with `fly secrets set -a bolik-api AWS_KEY=one`


## Android

All instructions are relative to `app` directory.

* Increment version in `pubspec.yaml`
    * Increment version before '+' and after. Number after '+' is called version code.
    * If prev is `1.0.0+1` then next is `1.0.1+2`
* Commit the change
* Make sure we build for all Android targets
    * See `cargoBuild` task in `android/app/build.gradle`
* `flutter build appbundle`
* Upload application bundle from `build/app/outputs/bundle/release/app-release.aab` to Play Console.

### Android: First time setup

* https://docs.flutter.dev/deployment/android

* Generate signing key with `keytool`
* Set up `android/key.properties` (do not commit)
    ```
    storePassword=<password from previous step>
    keyPassword=<password from previous step>
    keyAlias=upload
    storeFile=/Users/roman/upload-keystore.jks
    ```
* Modify `android/app/build.gradle` to load signing key and use release signing config


## iOS

All instructions are relative to `app` directory.

* Increment version in `pubspec.yaml`
    * Increment version before '+' and after. Number after '+' is called version code.
    * If prev is `1.0.0+1` then next is `1.0.1+2`
* Commit the change
* `flutter build ipa`
* Open Transporter mac app and upload `build/ios/ipa/*.ipa`
* Wait for Apple to build the app (they will send an email with build results)
* Distribute the build to testers in https://appstoreconnect.apple.com


## Mac

All instructions are relative to `app` directory.

* Increment version in `pubspec.yaml`
    * Increment version before '+' and after. Number after '+' is called version code.
    * If prev is `1.0.0+1` then next is `1.0.1+2`
* Commit the change
* `flutter build macos`
* `open macos/Runner.xcworkspace` and "Product --> Archive"
    * It is important to open `.xcworkspace` and not `.xcodeproj`![^mac-xcworkspace]
* Create an archive "Product --> Archive", ignore Xcode Cloud.
    * If XCode can't find cargo then you need to extend PATH env var
    * In `native/native.xcodeproj/project.pbxproj` find a line that starts with ` script = `
    * That line will contain PATH override. Include the directory where cargo is installed.
* "Validate App" *(this step can be skipped as it will be done in "Distribute App" step)*
    * Automatically manage signing
* "Distribute App"
    * App Store Connect
    * Upload
* Wait for Apple to build the app (they will send an email with build results)
* Distribute the build to testers in https://appstoreconnect.apple.com

[^mac-xcworkspace]: https://github.com/flutter/flutter/issues/114314#issuecomment-1315911977

### Mac Tips

* You can see existing archives in XCode: "Window --> Organizer"

