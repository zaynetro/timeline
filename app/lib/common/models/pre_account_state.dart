import 'package:timeline/bridge_generated.dart';
import 'package:timeline/common/models/dispatcher.dart';

/// State of the app before account is created.
class PreAccountState {
  final AppEventDispatcher dispatcher;
  final Native native;

  PreAccountState(this.dispatcher, this.native);

  Future<void> createAccount(String? name) async {
    final view = await native.createAccount(name: name);
    dispatcher
        .dispatch(OutputEvent.postAccount(PostAccountPhase(accView: view)));
  }

  Future<String> getDeviceShare() async {
    return native.getDeviceShare();
  }

  syncBackend() {
    native.sync();
  }
}
