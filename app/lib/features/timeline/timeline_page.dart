import 'dart:async';

import 'package:flutter/foundation.dart' show kDebugMode;
import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:provider/provider.dart';
import 'package:timeline/bridge_generated.dart';
import 'package:timeline/common/models/app_state.dart';
import 'package:timeline/common/models/phase_info.dart';
import 'package:timeline/features/card/card_page.dart';
import 'package:timeline/features/timeline/card_preview.dart';
import 'package:timeline/features/timeline/sidebar.dart';
import 'package:timeline/routes.dart';

class TimelinePage extends StatefulWidget {
  const TimelinePage({Key? key}) : super(key: key);

  @override
  State<TimelinePage> createState() => _TimelinePageState();
}

class _TimelinePageState extends State<TimelinePage>
    with WidgetsBindingObserver {
  List<AccLabel> selectedLabels = [];
  late final Timer _syncTimer;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);

    // Sync periodically when timeline page is active.
    final state = context.read<AppState>();
    _syncTimer = Timer.periodic(const Duration(seconds: 60), (timer) {
      if (ModalRoute.of(context)?.isCurrent ?? false) {
        state.syncBackend();
      }
    });
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    _syncTimer.cancel();
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.resumed) {
      // Sync only if timeline page is active
      if (ModalRoute.of(context)?.isCurrent ?? false) {
        final state = context.read<AppState>();
        state.syncBackend();
      }
    }
  }

  void selectLabelIds(List<AccLabel> labels) {
    setState(() => selectedLabels = labels);
  }

  bool _showingBin() {
    return selectedLabels.isNotEmpty && selectedLabels[0].id == deletedLabelId;
  }

  Widget _buildTitle() {
    if (selectedLabels.isNotEmpty) {
      var icon = Icons.label_outline;
      if (_showingBin()) {
        icon = Icons.delete_outline;
      }

      return Row(children: [
        Icon(icon),
        const SizedBox(width: 8),
        Flexible(child: Text(selectedLabels.map((l) => l.name).join(', '))),
        IconButton(
          onPressed: () => selectLabelIds([]),
          icon: const Icon(Icons.clear),
          tooltip: 'Clear search',
        )
      ]);
    }

    return const Text(
      'Bolik Timeline',
      style: TextStyle(color: Colors.black),
    );
  }

  @override
  Widget build(BuildContext context) {
    final state = context.read<AppState>();
    final showingBin = _showingBin();

    return Scaffold(
      appBar: AppBar(
        title: _buildTitle(),
        centerTitle: selectedLabels.isEmpty,
        actions: [
          if (!showingBin)
            ChangeNotifierProvider.value(
              value: state.notifications,
              child: _NotificationsButton(),
            ),
          if (showingBin)
            PopupMenuButton<_AppBarActions>(
              tooltip: 'Options',
              onSelected: (action) async {
                if (action == _AppBarActions.emptyBin) {
                  await state.native.emptyBin();
                  state.timeline.refresh();
                }
              },
              itemBuilder: (context) => [
                PopupMenuItem(
                  value: _AppBarActions.emptyBin,
                  child: Row(children: const [
                    Icon(Icons.delete_forever),
                    SizedBox(width: 16),
                    Text('Empty bin')
                  ]),
                ),
              ],
            ),
        ],
      ),
      drawer: TimelineDrawer(
        onSelectLabels: selectLabelIds,
        selectedLabels: selectedLabels,
      ),
      body: Column(
        children: [
          if (kDebugMode) _ColorSchemePalette(),
          Expanded(
            child: ChangeNotifierProvider.value(
              value: state.timeline,
              child: _TimelineList(selectedLabels: selectedLabels),
            ),
          ),
        ],
      ),
      floatingActionButton: _FloatingButton(selectedLabels: selectedLabels),
    );
  }
}

class _TimelineList extends StatelessWidget {
  final List<AccLabel> selectedLabels;
  // index to timeline item count mapping
  final Map<int, int> _daysCache = {};

  _TimelineList({super.key, required this.selectedLabels});

  @override
  Widget build(BuildContext context) {
    final timeline = context.watch<Timeline>();
    final selectedLabelIds = selectedLabels.map((l) => l.id).toList();

    return FutureBuilder(
        future: timeline.days(labelIds: selectedLabelIds),
        builder: (BuildContext context, AsyncSnapshot<List<String>> snapshot) {
          if (snapshot.hasError) {
            logger.error('Cannot fetch days ${snapshot.error}');
            return const Text('Oh noo..');
          } else if (!snapshot.hasData) {
            return const SizedBox();
          }

          final days = snapshot.data!;

          if (days.isEmpty) {
            return const Center(child: Text('Empty timeline'));
          }

          return ListView.builder(
            itemCount: days.length,
            itemBuilder: ((context, index) {
              final day = days[index];
              return FutureBuilder(
                  future: timeline.byDay(day, labelIds: selectedLabelIds),
                  builder: (BuildContext context,
                      AsyncSnapshot<TimelineDay> snapshot) {
                    final date = DateTime.parse(day);
                    if (snapshot.hasError) {
                      return Text('${days[index]}: oh noo...');
                    } else if (!snapshot.hasData) {
                      final count = _daysCache[index] ?? 0;
                      return _DayGroup(date,
                          cards: const [], placeholderCount: count);
                    }

                    final timelineDay = snapshot.data!;
                    _daysCache[index] = timelineDay.cards.length;
                    return _DayGroup(date, cards: timelineDay.cards);
                  });
            }),
          );
        });
  }
}

class _DayGroup extends StatelessWidget {
  final DateTime date;
  final List<CardView> cards;
  final int? placeholderCount;

  const _DayGroup(this.date,
      {super.key, required this.cards, this.placeholderCount});

  List<Widget> _previewItems(BuildContext context) {
    if (placeholderCount != null) {
      final colorScheme = Theme.of(context).colorScheme;
      // Fill with placeholders
      return List.generate(
          placeholderCount!,
          (index) => Container(
                color: colorScheme.primaryContainer,
              ));
    }

    return cards.map((card) => CardPreviewStack(card)).toList();
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(horizontal: 8),
      child: Column(crossAxisAlignment: CrossAxisAlignment.start, children: [
        // Day
        Padding(
          padding: const EdgeInsets.only(left: 8, bottom: 16),
          child: Text(
            DateFormat.yMMMMd().format(date),
            style: Theme.of(context).textTheme.titleMedium,
          ),
        ),
        // Previews
        GridView.custom(
          physics: const NeverScrollableScrollPhysics(),
          shrinkWrap: true,
          // padding: const EdgeInsets.all(20),
          gridDelegate: const SliverGridDelegateWithMaxCrossAxisExtent(
            maxCrossAxisExtent: cardPreviewMaxHeight,
            childAspectRatio: 4 / 5,
            crossAxisSpacing: 8,
            mainAxisSpacing: 8,
          ),
          childrenDelegate: SliverChildListDelegate(_previewItems(context)),
        ),
        const SizedBox(height: 16),
      ]),
    );
  }
}

class _FloatingButton extends StatelessWidget {
  final List<AccLabel> selectedLabels;
  const _FloatingButton({Key? key, required this.selectedLabels})
      : super(key: key);

  @override
  Widget build(BuildContext context) {
    final state = context.read<AppState>();

    return FloatingActionButton.extended(
      label: const Text('Create'),
      onPressed: () async {
        final card = await state.createCard();
        Navigator.pushNamed(context, BolikRoutes.card,
            arguments: CardPageArguments(
              card,
              template: StartTemplate.pick,
              selectedLabelIds: selectedLabels.map((l) => l.id).toList(),
            ));
      },
      tooltip: 'Create a document',
      icon: const Icon(Icons.add),
    );
  }
}

// Show current theme colors
class _ColorSchemePalette extends StatelessWidget {
  Widget _buildSingle(int i, Color color) {
    return Container(
      width: 60,
      height: 60,
      color: color,
      alignment: AlignmentDirectional.center,
      child: Text('$i'),
    );
  }

  @override
  Widget build(BuildContext context) {
    final scheme = Theme.of(context).colorScheme;
    final colors = [
      /* 0 */ scheme.primary,
      /* 1 */ scheme.primaryContainer,
      /* 2 */ scheme.secondaryContainer,
      /* 3 */ scheme.surface,
      /* 4 */ scheme.surfaceTint,
      /* 5 */ scheme.surfaceVariant,
      /* 6 */ scheme.tertiary,
      /* 7 */ scheme.tertiaryContainer,
      /* 8 */ scheme.background,
      /* 9 */ scheme.inversePrimary,
      /* 10 */ scheme.inverseSurface,
    ];

    return Wrap(
      children: colors
          .asMap()
          .entries
          .map((e) => _buildSingle(e.key, e.value))
          .toList(),
    );
  }
}

enum _AppBarActions {
  emptyBin,
}

class _NotificationsButton extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    final notifications = context.watch<Notifications>();

    return IconButton(
      onPressed: () {
        Navigator.pushNamed(context, BolikRoutes.notifications);
      },
      tooltip: 'Notifications',
      icon: Badge.count(
        count: notifications.ids.length,
        isLabelVisible: notifications.ids.isNotEmpty,
        child: const Icon(Icons.notifications),
      ),
    );
  }
}
