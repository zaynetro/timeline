import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:timeline/bridge_generated.dart';
import 'package:timeline/common/models/app_state.dart';
import 'package:timeline/common/models/phase_info.dart';
import 'package:timeline/features/card/card_edit.dart';
import 'package:timeline/features/card/card_page.dart';

const contactRequestPrefix = "contact-request/";
const cardSharePrefix = "card-share/";

class NotificationsPage extends StatelessWidget {
  const NotificationsPage({super.key});

  @override
  Widget build(BuildContext context) {
    final appState = context.read<AppState>();
    return Scaffold(
      appBar: AppBar(
        title: const Text('Notifications'),
      ),
      body: ChangeNotifierProvider.value(
        value: appState.notifications,
        child: _NotificationsPageBody(),
      ),
    );
  }
}

class _NotificationsPageBody extends StatefulWidget {
  @override
  State<StatefulWidget> createState() => _NotificationsPageBodyState();
}

class _NotificationsPageBodyState extends State<_NotificationsPageBody> {
  final Map<String, ProfileView> _profiles = {};
  final Set<String> _processingIds = {};
  final Map<String, CardView> _cards = {};

  @override
  void initState() {
    super.initState();
    _load();
  }

  void _load() async {
    final appState = context.read<AppState>();
    final notifications = context.read<Notifications>();

    // Load profiles
    final list = await appState.native.listProfiles();
    _profiles.clear();
    for (var profile in list) {
      _profiles[profile.accountId] = profile;
    }
    setState(() {});

    // Load all cards
    for (var id in notifications.ids) {
      try {
        if (id.startsWith(cardSharePrefix)) {
          final cardId = id.split('/')[1];
          final card = await appState.native.getCard(cardId: cardId);
          _cards[card.id] = card;
          setState(() {});
        }
      } catch (e) {
        logger.warn("Failed to get card: $e");
      }
    }
  }

  void _process(String id, {required bool accept, bool? shouldPop}) async {
    final notifications = context.read<Notifications>();
    setState(() {
      _processingIds.add(id);
    });

    try {
      if (accept) {
        await notifications.accept(id);
      } else {
        await notifications.ignore(id);
      }

      if (shouldPop == true && mounted) {
        Navigator.pop(context);
      }
    } finally {
      setState(() {
        _processingIds.remove(id);
      });
    }
  }

  void _accept(String id, {bool? shouldPop}) =>
      _process(id, accept: true, shouldPop: shouldPop);

  void _ignore(String id, {bool? shouldPop}) =>
      _process(id, accept: false, shouldPop: shouldPop);

  Widget contactRequest(String id) {
    final contactId = id.split('/')[1];
    final name =
        _profiles[contactId]?.name ?? 'Acc #${contactId.substring(0, 6)}';
    final processing = _processingIds.contains(id);

    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        ListTile(
          leading: const Icon(Icons.person),
          title: Text(
            '$name wants to connect.',
            style: const TextStyle(fontWeight: FontWeight.w400),
          ),
        ),
        Padding(
          padding: const EdgeInsets.only(bottom: 8, right: 8),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.end,
            children: [
              TextButton(
                onPressed: processing ? null : () => _accept(id),
                child: const Text('Add to contacts'),
              ),
              const SizedBox(width: 16),
              TextButton(
                onPressed: processing ? null : () => _ignore(id),
                child:
                    const Text('Ignore', style: TextStyle(color: Colors.grey)),
              ),
            ],
          ),
        ),
      ],
    );
  }

  Widget cardShare(String id) {
    final cardId = id.split('/')[1];
    final card = _cards[cardId];
    if (card == null) {
      return const SizedBox();
    }

    final appState = context.read<AppState>();
    final accEdit = context.read<AccEdit>();
    final contacts = accEdit.view.contacts;
    final processing = _processingIds.contains(id);
    final names =
        card.acl.accounts.where((e) => e.accountId != accEdit.view.id).map((e) {
      // Find the name
      for (var c in contacts) {
        if (c.accountId == e.accountId) {
          return c.name;
        }
      }

      return _profiles[e.accountId]?.name ?? 'Unknown';
    }).toList();
    names.sort((a, b) => a.compareTo(b));

    String namesStr;
    if (names.length >= 4) {
      const take = 2;
      namesStr = names.take(take).join(', ');
      namesStr += ' and ${names.length - take} others';
    } else {
      namesStr = names.join(', ');
    }

    return Column(
      mainAxisSize: MainAxisSize.min,
      children: [
        ListTile(
          leading: const Icon(Icons.view_timeline),
          title: Text(
            'New card share. Join $namesStr.',
            style: const TextStyle(fontWeight: FontWeight.w400),
          ),
        ),
        Padding(
          padding: const EdgeInsets.only(bottom: 8, right: 8),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.end,
            children: [
              TextButton(
                onPressed: processing
                    ? null
                    : () {
                        Navigator.push(
                          context,
                          MaterialPageRoute(
                            builder: (context) => ChangeNotifierProvider(
                              create: (_) => TimelineCardEdit(
                                card,
                                appState.dispatcher,
                                appState.native,
                                cardPreview: true,
                              ),
                              child: _CardPreview(
                                onAccept: processing
                                    ? null
                                    : () => _accept(id, shouldPop: true),
                                onIgnore: processing
                                    ? null
                                    : () => _ignore(id, shouldPop: true),
                              ),
                            ),
                          ),
                        );
                      },
                child: const Text('Preview card'),
              ),
              // TODO: do we need to show ignore here?
              // const SizedBox(width: 16),
              // TextButton(
              //   onPressed: processing ? null : () => _ignore(id),
              //   child:
              //       const Text('Ignore', style: TextStyle(color: Colors.grey)),
              // ),
            ],
          ),
        ),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    final notifications = context.watch<Notifications>();

    final list = notifications.ids.isEmpty
        ? const Text('Nothing new...')
        : ListView.builder(
            itemCount: notifications.ids.length,
            itemBuilder: (context, index) {
              final id = notifications.ids[index];
              if (id.startsWith(contactRequestPrefix)) {
                return Card(child: contactRequest(id));
              } else if (id.startsWith(cardSharePrefix)) {
                return Card(child: cardShare(id));
              } else {
                return const SizedBox();
              }
            },
          );

    return Center(
      child: Container(
        alignment: Alignment.center,
        constraints: const BoxConstraints(maxWidth: 500),
        padding: const EdgeInsets.symmetric(horizontal: 8),
        child: list,
      ),
    );
  }
}

class _CardPreview extends StatelessWidget {
  final Function()? onAccept;
  final Function()? onIgnore;

  const _CardPreview(
      {super.key, required this.onAccept, required this.onIgnore});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(title: const Text('Preview')),
      body: const CardContent(),
      bottomNavigationBar: SafeArea(
        child: Padding(
          padding: const EdgeInsets.only(left: 16, right: 16, bottom: 16),
          child: Row(
            mainAxisAlignment: MainAxisAlignment.end,
            children: [
              TextButton(
                onPressed: onAccept,
                child: const Text('Add to timeline'),
              ),
              const SizedBox(width: 16),
              TextButton(
                onPressed: onIgnore,
                child: const Text(
                  'Ignore',
                  style: TextStyle(color: Colors.grey),
                ),
              ),
            ],
          ),
        ),
      ),
    );
  }
}
