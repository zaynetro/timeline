import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:timeline/bridge_generated.dart';
import 'package:timeline/common/models/dispatcher.dart';
import 'package:timeline/common/models/phase_info.dart';
import 'package:timeline/routes.dart';

void main() async {
  WidgetsFlutterBinding.ensureInitialized();

  final info = await setupDevice(AppEventDispatcher());
  runApp(BolikApp(info: info));
}

class BolikApp extends StatefulWidget {
  final DevicePhaseInfo info;
  const BolikApp({super.key, required this.info});

  @override
  State<StatefulWidget> createState() => _BolikAppState();
}

class _BolikAppState extends State<BolikApp> {
  @override
  void initState() {
    super.initState();
    widget.info.dispatcher.addListener(_eventListener);
  }

  @override
  void dispose() {
    super.dispose();
    widget.info.dispatcher.removeListener(_eventListener);
  }

  void _eventListener(OutputEvent event) async {
    if (event is OutputEvent_PostAccount) {
      if (widget.info.appState.value == null) {
        widget.info.onPostAccount(event.accView);

        // Replace all routes with a timeline page
        BolikRoutes.rootNav.currentState!
            .pushNamedAndRemoveUntil(BolikRoutes.timeline, (route) => false);
      }
    } else if (event is OutputEvent_LogOut) {
      await widget.info.onLogout();

      // Replace all routes with a index page
      BolikRoutes.rootNav.currentState!
          .pushNamedAndRemoveUntil(BolikRoutes.index, (route) => false);
    }
  }

  Widget _injectProviders({required Widget child}) {
    return MultiProvider(providers: [
      ValueListenableProvider.value(value: widget.info.preAccount),
      ValueListenableProvider.value(value: widget.info.appState),
      ChangeNotifierProvider.value(value: widget.info.account),
    ], child: child);
  }

  @override
  Widget build(BuildContext context) {
    String initialRoute = BolikRoutes.index;
    if (widget.info.sdkFatalError) {
      initialRoute = BolikRoutes.sdkError;
    } else if (widget.info.appState.value != null) {
      initialRoute = BolikRoutes.timeline;
    }

    return _injectProviders(
      child: MaterialApp(
        title: 'Bolik Timeline',
        theme: ThemeData(
          colorSchemeSeed: Colors.orange,
          useMaterial3: true,
        ),
        initialRoute: initialRoute,
        navigatorKey: BolikRoutes.rootNav,
        onGenerateInitialRoutes: (initialRoute) {
          final route = rootOnGenerateRoute(RouteSettings(name: initialRoute))!;
          return [route];
        },
        onGenerateRoute: rootOnGenerateRoute,
      ),
    );
  }
}
