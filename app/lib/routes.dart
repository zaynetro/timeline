import 'dart:io';

import 'package:flutter/material.dart';
import 'package:timeline/features/account/account_page.dart';
import 'package:timeline/features/account/contacts.dart';
import 'package:timeline/features/account_setup/account_setup_page.dart';
import 'package:timeline/features/account_setup/sdk_error_page.dart';
import 'package:timeline/features/card/card_page.dart';
import 'package:timeline/features/card/collaborators.dart';
import 'package:timeline/features/card/labels.dart';
import 'package:timeline/features/connect_device/connect_device_page.dart';
import 'package:timeline/features/import/export_page.dart';
import 'package:timeline/features/import/import_page.dart';
import 'package:timeline/features/labels/acc_labels.dart';
import 'package:timeline/features/log_view/log_view.dart';
import 'package:timeline/features/notifications/notifications_page.dart';
import 'package:timeline/features/timeline/timeline_page.dart';

/// A statically typed list of routes
class BolikRoutes {
  static final GlobalKey<NavigatorState> rootNav = GlobalKey();
  static final GlobalKey<NavigatorState> dialogNav = GlobalKey();

  // Unauthenticated routes
  static const index = '/';
  static const sdkError = '/sdk-error';

  // Authenticated routes
  static const timeline = '/timeline';
  static const card = '/card';
  static const cardCollaborators = '/card/collaborators';
  static const cardLabels = '/card/labels';
  static const logs = '/logs';
  static const acc = '/account';
  static const accEdit = '/account/edit';
  static const accDevicesAdd = '/account/devices/add';
  static const accContacts = '/account/contacts';
  static const accContactsAdd = '/account/contacts/add';
  static const accContactsEdit = '/account/contacts/edit';
  static const accLabels = '/account/labels';
  static const notifications = '/notifications';
  static const importData = '/import';
  static const exportData = '/export';
  static const logout = '/logout';

  /// Navigate to the previous page
  static void goBack() {
    if (dialogNav.currentState?.canPop() ?? false) {
      // If we are inside a dialog then use dialog's navigator
      dialogNav.currentState!.pop();
    } else {
      // If we reached the first dialog page or outside the dialog then use root navigator
      rootNav.currentState!.pop();
    }
  }
}

Widget Function(BuildContext context)? _pageBuilder(RouteSettings settings) {
  switch (settings.name) {
    // Unauthenticated routes
    case BolikRoutes.index:
      return (context) => const AccountSetupPage();
    case BolikRoutes.sdkError:
      return (context) => const SdkErrorPage();

    // Authenticated routes

    // Card
    case BolikRoutes.timeline:
      return (context) => const TimelinePage();
    case BolikRoutes.card:
      return (context) => const CardPage();
    case BolikRoutes.cardCollaborators:
      return (context) => const CollaboratorsPage();
    case BolikRoutes.cardLabels:
      return (context) => const CardLabelsPicker();

    // Account
    case BolikRoutes.acc:
      return (context) => const AccountPage();
    case BolikRoutes.accEdit:
      return (context) => const EditAccountPage();
    case BolikRoutes.accDevicesAdd:
      return (context) => const ConnectDevicePage();
    case BolikRoutes.accContacts:
      var args = ContactsListArgs();
      if (settings.arguments is ContactsListArgs) {
        args = settings.arguments as ContactsListArgs;
      }
      return (context) => ContactsListPage(args: args);
    case BolikRoutes.accContactsAdd:
      return (context) => const AddContactPage();
    case BolikRoutes.accContactsEdit:
      return (context) =>
          EditContactPage(args: settings.arguments as EditContactArgs);
    case BolikRoutes.accLabels:
      return (context) => const AccLabelsPage();

    case BolikRoutes.notifications:
      return (context) => const NotificationsPage();

    // Misc
    case BolikRoutes.logs:
      return (context) => const LogViewPage();
    case BolikRoutes.logout:
      return (context) => const LogOutPage();
    case BolikRoutes.importData:
      return (context) => const ImportPage();
    case BolikRoutes.exportData:
      return (context) => const ExportPage();
    default:
      return null;
  }
}

Route<T>? rootOnGenerateRoute<T>(RouteSettings settings) {
  final pageBuilder = _pageBuilder(settings);
  if (pageBuilder == null) {
    return null;
  }

  return MaterialPageRoute(builder: pageBuilder, settings: settings);
}

Route<T>? _dialogOnGenerateRoute<T>(RouteSettings settings) {
  final pageBuilder = _pageBuilder(settings);
  if (pageBuilder == null) {
    return null;
  }

  // We don't use MaterialPageRoute inside dialogs because of animation glitches.
  return PageRouteBuilder(
    pageBuilder: (context, animation, secondaryAnimation) =>
        pageBuilder(context),
    transitionsBuilder: (context, animation, secondaryAnimation, child) {
      return FadeTransition(
        opacity: animation,
        child: child,
      );
    },
  );
}

/// Open a dialog with a navigator. Pages will be able to navigate inside an open dialog.
Future<T?> showDialogNav<T>({
  required BuildContext context,
  required String initialRoute,
  required Widget Function(BuildContext context, Widget child) builder,
}) {
  final screenSize = MediaQuery.of(context).size;
  if (screenSize.width < 700) {
    // For mobiles we just navigate to the page
    // There is a bit of trickery here needed to support context injections.
    final settings = RouteSettings(name: initialRoute);
    final pageBuilder = _pageBuilder(settings)!;
    final route = MaterialPageRoute<T>(
      builder: (context) => builder(context, pageBuilder(context)),
      settings: settings,
    );

    return Navigator.push(context, route);
  }

  final dialogHeight = MediaQuery.of(context).size.height;
  final child = Dialog(
    child: SizedBox(
      width: 500,
      height: dialogHeight,
      child: Navigator(
        key: BolikRoutes.dialogNav,
        initialRoute: initialRoute,
        onGenerateInitialRoutes: (state, initialRoute) {
          final route =
              _dialogOnGenerateRoute(RouteSettings(name: initialRoute))!;
          return [route];
        },
        onGenerateRoute: _dialogOnGenerateRoute,
      ),
    ),
  );

  return showDialog<T>(
    context: context,
    builder: (context) => builder(context, child),
  );
}
