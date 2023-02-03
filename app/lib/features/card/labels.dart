import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:timeline/common/models/app_state.dart';
import 'package:timeline/features/card/card_edit.dart';
import 'package:timeline/routes.dart';

class CardLabelsPicker extends StatefulWidget {
  const CardLabelsPicker({super.key});

  @override
  State<StatefulWidget> createState() => _CardLabelsPickerState();
}

class _CardLabelsPickerState extends State<CardLabelsPicker> {
  final _textController = TextEditingController();

  @override
  void dispose() {
    _textController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final accEdit = context.watch<AccEdit>();
    final cardEdit = context.watch<TimelineCardEdit>();

    final accountLabels = accEdit.view.labels;
    final text = _textController.text;
    final textFilter = text.toLowerCase();
    final labelMissing =
        accountLabels.where((l) => l.name.toLowerCase() == textFilter).isEmpty;
    final showAddLabel = text.isNotEmpty && labelMissing;
    final filteredLabels =
        accountLabels.where((l) => l.name.toLowerCase().contains(textFilter));

    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(
        leading: IconButton(
          onPressed: () {
            BolikRoutes.goBack();
          },
          tooltip: 'Cancel',
          icon: const Icon(Icons.close),
        ),
        title: TextField(
          controller: _textController,
          decoration: const InputDecoration(
            hintText: 'Enter label name',
          ),
          onChanged: (content) {
            // Rebuild the widget to filter list items
            setState(() {});
          },
          textCapitalization: TextCapitalization.sentences,
        ),
      ),
      body: Column(
        children: [
          if (showAddLabel)
            ListTile(
              onTap: () async {
                _textController.text = '';
                final label = await accEdit.createLabel(name: text);
                cardEdit.addLabel(label.id);
              },
              leading: Icon(Icons.add, color: theme.colorScheme.primary),
              title: Text(
                'Create "$text"',
                style: TextStyle(color: theme.colorScheme.primary),
              ),
              hoverColor: theme.colorScheme.primary.withOpacity(0.08),
            ),
          const SizedBox(height: 8),
          Expanded(
            child: ListView(
              // shrinkWrap: true,
              children: filteredLabels
                  .map((l) => CheckboxListTile(
                      title: Text(l.name),
                      secondary: const Icon(Icons.label_outline),
                      value: cardEdit.card.labels
                          .any((cardLabel) => cardLabel.id == l.id),
                      onChanged: (checked) {
                        checked == true
                            ? cardEdit.addLabel(l.id)
                            : cardEdit.removeLabel(l.id);
                      }))
                  .toList(),
            ),
          ),
        ],
      ),
    );
  }
}
