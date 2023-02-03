import Cocoa
import FlutterMacOS

@NSApplicationMain
class AppDelegate: FlutterAppDelegate {
  override func applicationShouldTerminateAfterLastWindowClosed(_ sender: NSApplication) -> Bool {
    // We need to show XCode that that code is used so that it is not stripped.
    // http://cjycode.com/flutter_rust_bridge/integrate/ios_headers.html
    let dummy = dummy_method_to_enforce_bundling()
    print(dummy)

    return true
  }
}
