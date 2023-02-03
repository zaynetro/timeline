import 'package:flutter/foundation.dart';
import 'package:timeline/bridge_generated.dart';
import 'package:timeline/common/models/dispatcher.dart';

class AppState {
  final Timeline timeline;
  final AppEventDispatcher dispatcher;
  final Native native;
  final Notifications notifications;

  AppState._(this.timeline, this.dispatcher, this.native, this.notifications);

  static AppState build(AppEventDispatcher dispatcher, Native native) {
    final timeline = Timeline(dispatcher, native);
    final notifications = Notifications(dispatcher, native);
    return AppState._(timeline, dispatcher, native, notifications);
  }

  Future<CardView> createCard() async {
    return native.createCard();
  }

  Future<CardView> findCard(String id) async {
    return native.getCard(cardId: id);
  }

  syncBackend() {
    native.sync();
  }

  /// Clear all local data
  Future<void> dangerousLogout() async {
    await native.logout();
    dispatcher.dispatch(const OutputEvent.logOut());
  }

  String getCurrentDeviceId() {
    return native.getCurrentDeviceId();
  }
}

class Timeline extends ChangeNotifier {
  final AppEventDispatcher _dispatcher;
  final Native native;

  Timeline(this._dispatcher, this.native) {
    _dispatcher.addListener<OutputEvent_TimelineUpdated>((_) {
      refresh();
    });

    _dispatcher.addListener<OutputEvent_DocUpdated>((_) {
      refresh();
    });
  }

  void refresh() {
    notifyListeners();
  }

  Future<List<String>> days({required List<String> labelIds}) async {
    // Use to test scrolling
    // final today = DateTime.now();
    // return List.generate(
    //   50,
    //   (index) => DateFormat('yyyy-MM-dd')
    //       .format(today.subtract(Duration(days: index))),
    // );

    return native.timelineDays(labelIds: labelIds);
  }

  Future<TimelineDay> byDay(String day,
      {required List<String> labelIds}) async {
    // Use to test scrolling
    // await Future.delayed(const Duration(seconds: 3));
    // final date = DateTime.parse(day).add(const Duration(hours: 6));
    // final count = date.day;

    // return List.generate(count, (index) {
    //   return TimelineItem(
    //     id: '$date-$index',
    //     createdAtSec: date.millisecondsSinceEpoch ~/ 1000,
    //     editedAtSec: date.millisecondsSinceEpoch ~/ 1000,
    //     content: TimelineCardPreview(CardDocPreview(
    //       text: '$date-$index',
    //       hasAttachment: false,
    //     )),
    //   );
    // });

    return native.timelineByDay(day: day, labelIds: labelIds);
  }
}

class Notifications extends ChangeNotifier {
  final AppEventDispatcher _dispatcher;
  final Native _native;
  final List<String> ids = [];

  Notifications(this._dispatcher, this._native) {
    _dispatcher.addListener<OutputEvent_Notification>((event) {
      ids.add(event.id);
      notifyListeners();
    });

    _dispatcher.addListener<OutputEvent_NotificationsUpdated>((event) {
      _load();
    });

    _load();
  }

  void _load() async {
    final fetched = await _native.listNotificationIds();
    ids.clear();
    ids.addAll(fetched);
    notifyListeners();
  }

  Future<void> accept(String id) async {
    await _native.acceptNotification(id: id);
    ids.remove(id);
    notifyListeners();
  }

  Future<void> ignore(String id) async {
    await _native.ignoreNotification(id: id);
    ids.remove(id);
    notifyListeners();
  }
}

class AccEdit extends ChangeNotifier {
  AccView view;
  final AppEventDispatcher _dispatcher;
  final Native native;

  AccEdit(this.view, this._dispatcher, this.native) {
    setView(view);

    _dispatcher.addListener<OutputEvent_AccUpdated>((event) {
      if (event.field0.id == view.id) {
        setView(event.field0);
        notifyListeners();
      }
    });
  }

  AccEdit.empty(AppEventDispatcher dispatcher, Native native)
      : this(
            AccView(
              id: 'empty',
              name: '',
              contacts: [],
              devices: [],
              labels: [],
              createdAtSec: 0,
            ),
            dispatcher,
            native);

  Future<void> editName(String name) async {
    final acc = await native.editName(name: name);
    setView(acc);
    notifyListeners();
  }

  Future<void> addContact(
      {required String accountId, required String name}) async {
    final acc = await native.addContact(
        contact: AccContact(accountId: accountId, name: name));
    setView(acc);
    notifyListeners();
  }

  Future<void> editContactName(
      {required String accountId, required String name}) async {
    final acc = await native.editContactName(accountId: accountId, name: name);
    setView(acc);
    notifyListeners();
  }

  Future<AccLabel> createLabel({required String name}) async {
    final res = await native.createAccLabel(name: name);
    setView(res.view);
    notifyListeners();
    return res.label;
  }

  Future<void> deleteLabel({required String labelId}) async {
    final acc = await native.deleteAccLabel(labelId: labelId);
    setView(acc);
    notifyListeners();
  }

  Future<String> linkDevice(String share) async {
    final newDeviceName = await native.linkDevice(share: share);
    refresh();
    return newDeviceName;
  }

  Future<void> removeDevice(String deviceId) async {
    final acc = await native.removeDevice(removeId: deviceId);
    setView(acc);
    notifyListeners();
  }

  Future<void> refresh() async {
    final acc = await native.getAccount();
    if (acc != null) {
      setView(acc);
      notifyListeners();
    }
  }

  setView(AccView acc) {
    view = acc;
    view.labels.sort((a, b) => a.name.compareTo(b.name));
    view.contacts.sort((a, b) => a.name.compareTo(b.name));
  }
}
