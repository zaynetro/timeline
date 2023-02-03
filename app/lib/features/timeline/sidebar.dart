import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:timeline/bridge_generated.dart';
import 'package:timeline/common/models/app_state.dart';
import 'package:timeline/common/models/phase_info.dart';
import 'package:timeline/routes.dart';

class TimelineDrawer extends StatelessWidget {
  final List<AccLabel> selectedLabels;
  final void Function(List<AccLabel> labels) onSelectLabels;

  const TimelineDrawer({
    super.key,
    required this.onSelectLabels,
    required this.selectedLabels,
  });

  @override
  Widget build(BuildContext context) {
    final accEdit = context.watch<AccEdit>();
    final accName = accEdit.view.name.isEmpty ? 'Hello!' : accEdit.view.name;
    final labels = accEdit.view.labels;
    final theme = Theme.of(context);

    return Drawer(
      child: CustomScrollView(
        slivers: [
          SliverSafeArea(
            sliver: SliverToBoxAdapter(
              child: Padding(
                padding: const EdgeInsets.only(
                    top: 20, left: 16, right: 8, bottom: 8),
                child: Text(accName, style: theme.textTheme.titleLarge),
              ),
            ),
          ),
          SliverList(
            delegate: SliverChildListDelegate([
              ListTile(
                onTap: () {
                  Navigator.pop(context);
                  showDialogNav(
                    context: context,
                    initialRoute: BolikRoutes.acc,
                    builder: (context, child) => child,
                  );
                },
                leading: const Icon(Icons.account_circle),
                title: const Text('My Account'),
              ),
              Divider(color: theme.colorScheme.tertiary),
              ListTile(
                selected: selectedLabels.isEmpty,
                selectedTileColor: theme.hoverColor,
                onTap: () {
                  onSelectLabels([]);
                  Navigator.pop(context);
                },
                leading: const Icon(Icons.view_timeline),
                title: const Text('Timeline'),
              ),
              ListTile(
                selected: false,
                selectedTileColor: theme.hoverColor,
                onTap: () {
                  Navigator.pop(context);
                  showDialogNav(
                    context: context,
                    initialRoute: BolikRoutes.accContacts,
                    builder: (context, child) => child,
                  );
                },
                leading: const Icon(Icons.contacts),
                title: const Text('Contacts'),
              ),
              Padding(
                padding:
                    const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
                child: Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    const Text('Labels:'),
                    TextButton(
                      onPressed: () {
                        showDialogNav(
                          context: context,
                          initialRoute: BolikRoutes.accLabels,
                          builder: (context, child) =>
                              ChangeNotifierProvider.value(
                            value: accEdit,
                            child: child,
                          ),
                        );
                      },
                      child: const Text('Edit'),
                    ),
                  ],
                ),
              ),
            ]),
          ),
          SliverList(
            delegate: SliverChildBuilderDelegate(
              (context, index) {
                final label = labels[index];

                return ListTile(
                  selected: selectedLabels.any((l) => l.id == label.id),
                  selectedTileColor: theme.hoverColor,
                  onTap: () {
                    onSelectLabels([label]);
                    Navigator.pop(context);
                  },
                  leading: const Icon(Icons.label_outline),
                  title: Text(label.name),
                );
              },
              childCount: labels.length,
            ),
          ),
          SliverList(
            delegate: SliverChildListDelegate([
              Divider(color: theme.colorScheme.tertiary),
              ListTile(
                selected: selectedLabels.any((l) => l.id == deletedLabelId),
                selectedTileColor: theme.hoverColor,
                onTap: () {
                  onSelectLabels(
                      [AccLabel(id: deletedLabelId, name: 'Deleted')]);
                  Navigator.pop(context);
                },
                leading: const Icon(Icons.delete_outline),
                title: const Text("Deleted"),
              ),
            ]),
          )
        ],
      ),
    );
  }
}
