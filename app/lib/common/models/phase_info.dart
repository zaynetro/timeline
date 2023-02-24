import 'dart:async';
import 'dart:ffi';
import 'dart:io';

import 'package:device_info_plus/device_info_plus.dart';
import 'package:flutter/foundation.dart';
import 'package:path_provider/path_provider.dart';
import 'package:timeline/bridge_generated.dart';
import 'package:timeline/common/models/app_state.dart';
import 'package:timeline/common/models/dispatcher.dart';
import 'package:timeline/common/models/logger.dart';
import 'package:timeline/common/models/pre_account_state.dart';

final logger = Logger(LogLevel.trace, stdOutput: kDebugMode);

const _base = 'native';
final _dylibPath = Platform.isWindows ? '$_base.dll' : 'lib$_base.so';
final _dylib = Platform.isIOS
    ? DynamicLibrary.process()
    : Platform.isMacOS
        ? DynamicLibrary.executable()
        : DynamicLibrary.open(_dylibPath);
final Native _native = NativeImpl(_dylib);

DeviceInfoPlugin deviceInfo = DeviceInfoPlugin();

// ID of a "delete" label.
final deletedLabelId = _native.getDeletedLabelId();

Future<String> _getAppSupportPath() async {
  const appSupportEnv = String.fromEnvironment('BOLIK_APP_SUPPORT_PATH');
  if (appSupportEnv.isNotEmpty) {
    return appSupportEnv;
  }

  final libDir = await getApplicationSupportDirectory();
  return libDir.path;
}

Future<void> _startNative(AppEventDispatcher dispatcher) async {
  final appSupportDir = await _getAppSupportPath();
  logger.addFileOutput(appSupportDir);

  logger.info("Set up native logs");
  _native.setupLogs().handleError((e) {
    logger.warn("Failed to set up native logs: $e");
  }).listen((logRow) {
    logger.nativeEvent(logRow);
  });

  final deviceName = await _getDeviceName();

  final String filesDir;
  if (Platform.isAndroid) {
    // TODO: we need to check whether external storage is available
    // https://developer.android.com/training/data-storage/shared
    filesDir = (await getExternalStorageDirectory())!.path;
  } else {
    // TODO: consider using getApplicationDocumentsDirectory
    filesDir = await _getAppSupportPath();
  }

  logger.info("Set up native module");
  await for (final event in _native.setup(
    appSupportDir: appSupportDir,
    filesDir: filesDir,
    deviceName: deviceName,
  )) {
    dispatcher.dispatch(event);
  }
}

/// Specifies what is the current app phase.
/// - PreAccount (no account is set up)
/// - PostAccount (has account)
class DevicePhaseInfo {
  AppEventDispatcher dispatcher;
  ValueNotifier<PreAccountState?> _preAccount;
  ValueNotifier<AppState?> _appState;
  AccEdit account;
  final bool sdkFatalError;

  DevicePhaseInfo._(this.dispatcher,
      {PreAccountState? preAccount,
      AppState? appState,
      required this.account,
      this.sdkFatalError = false})
      : _preAccount = ValueNotifier(preAccount),
        _appState = ValueNotifier(appState);

  DevicePhaseInfo.preAccount(AppEventDispatcher dispatcher)
      : this._(dispatcher,
            preAccount: PreAccountState(dispatcher, _native),
            account: AccEdit.empty(dispatcher, _native));
  DevicePhaseInfo.appState(AppEventDispatcher dispatcher, AccView acc)
      : this._(dispatcher,
            appState: AppState.build(dispatcher, _native),
            account: AccEdit(acc, dispatcher, _native));

  /// Device was connected to account
  void onPostAccount(AccView acc) {
    account.setView(acc);
    _appState.value = AppState.build(dispatcher, _native);
  }

  Future<void> onLogout() async {
    _appState.value = null;

    final info = await setupDevice(dispatcher);
    _preAccount.value = info.preAccount.value;
  }

  ValueListenable<PreAccountState?> get preAccount => _preAccount;
  ValueListenable<AppState?> get appState => _appState;
}

/// Start native module and configure device phase info.
Future<DevicePhaseInfo> setupDevice(AppEventDispatcher dispatcher) async {
  final completer = Completer<DevicePhaseInfo>();

  void listener(OutputEvent event) {
    if (event is OutputEvent_PreAccount) {
      completer.complete(DevicePhaseInfo.preAccount(dispatcher));
    } else if (event is OutputEvent_PostAccount) {
      completer
          .complete(DevicePhaseInfo.appState(dispatcher, event.accView));
    }
  }

  dispatcher.addListener(listener);

  void run() async {
    try {
      await _startNative(dispatcher);
    } catch (e, st) {
      logger.error('Cannot setup native module: $e $st');
      completer.complete(DevicePhaseInfo._(dispatcher,
          sdkFatalError: true, account: AccEdit.empty(dispatcher, _native)));
    } finally {
      dispatcher.removeListener(listener);
    }
  }

  run();

  return completer.future;
}

Future<String> _getDeviceName() async {
  if (Platform.isAndroid) {
    final info = await deviceInfo.androidInfo;
    return '${info.manufacturer} ${info.model}';
  } else if (Platform.isIOS) {
    final info = await deviceInfo.iosInfo;
    return '${info.name} ${info.systemVersion}';
  } else if (Platform.isLinux) {
    return (await deviceInfo.linuxInfo).name;
  } else if (Platform.isMacOS) {
    final info = await deviceInfo.macOsInfo;
    return '${info.computerName} ${info.model}';
  } else if (Platform.isWindows) {
    return (await deviceInfo.windowsInfo).computerName;
  }

  return 'Unknown device';
}
