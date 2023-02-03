import UIKit
import Flutter

@UIApplicationMain
@objc class AppDelegate: FlutterAppDelegate {
  override func application(
    _ application: UIApplication,
    didFinishLaunchingWithOptions launchOptions: [UIApplication.LaunchOptionsKey: Any]?
  ) -> Bool {
    // We need to show XCode that that code is used so that it is not stripped.
    // http://cjycode.com/flutter_rust_bridge/integrate/ios_headers.html
    let dummy = dummy_method_to_enforce_bundling()
    NSLog("Dummy: %d", dummy)
    GeneratedPluginRegistrant.register(with: self)
    return super.application(application, didFinishLaunchingWithOptions: launchOptions)
  }
}
