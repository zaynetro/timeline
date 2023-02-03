import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:timeline/bridge_generated.dart';
import 'package:timeline/common/models/app_state.dart';
import 'package:timeline/features/card/card_edit.dart';

class MoveToBinDialog extends StatefulWidget {
  const MoveToBinDialog({super.key});

  @override
  State<StatefulWidget> createState() => _MoveToBinDialogState();
}

class _MoveToBinDialogState extends State<MoveToBinDialog> {
  bool deleteAll = false;

  @override
  Widget build(BuildContext context) {
    final cardEdit = context.read<TimelineCardEdit>();
    final accEdit = context.read<AccEdit>();
    final acl = cardEdit.card.acl;
    final sharedWithOthers = acl.accounts.length > 1;
    final isAdmin = acl.accounts.any((e) =>
        (e.accountId == accEdit.view.id) && (e.rights == AclRights.Admin));

    return AlertDialog(
      title: const Text("Delete card"),
      content: Column(
        crossAxisAlignment: CrossAxisAlignment.stretch,
        mainAxisSize: MainAxisSize.min,
        children: [
          const Padding(
            padding: EdgeInsets.symmetric(horizontal: 8),
            child: Text('Are you sure you want to delete this card?'),
          ),
          const SizedBox(height: 16),
          if (sharedWithOthers && isAdmin)
            InkWell(
              onTap: () {
                setState(() => deleteAll = !deleteAll);
              },
              hoverColor: Colors.transparent,
              child: Row(children: [
                Checkbox(
                    value: deleteAll,
                    onChanged: (value) {
                      setState(() => deleteAll = value ?? false);
                    }),
                const SizedBox(width: 8),
                const Flexible(child: Text('Also delete for other accounts')),
              ]),
            ),
        ],
      ),
      actions: [
        TextButton(
          onPressed: () {
            Navigator.pop(context);
          },
          child: const Text('Cancel'),
        ),
        TextButton(
          onPressed: () async {
            final cardEdit = context.read<TimelineCardEdit>();
            final appState = context.read<AppState>();
            if (deleteAll) {
              await cardEdit.moveToBinAll();
            } else {
              await cardEdit.moveToBin();
            }
            appState.timeline.refresh();

            if (!mounted) return;
            // Close dialog
            Navigator.pop(context);
            // Return to timeline page
            Navigator.pop(context);

            ScaffoldMessenger.of(context).showSnackBar(
                const SnackBar(content: Text('Card moved to bin')));
          },
          child: const Text('Delete'),
        ),
      ],
    );
  }
}
