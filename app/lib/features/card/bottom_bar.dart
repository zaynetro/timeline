import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:fleather/fleather.dart';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:timeline/common/models/app_state.dart';
import 'package:timeline/features/card/card_edit.dart';
import 'package:timeline/features/card/toolbar.dart';
import 'package:timeline/routes.dart';

class CardBottomBar extends StatelessWidget {
  final void Function(FileType) onPickFile;
  final Function(bool isOpen) onFormatToolbar;

  const CardBottomBar({
    Key? key,
    required this.onPickFile,
    required this.onFormatToolbar,
  }) : super(key: key);

  @override
  Widget build(BuildContext context) {
    final accEdit = context.read<AccEdit>();
    final cardEdit = context.read<TimelineCardEdit>();
    final dense = Platform.isAndroid || Platform.isIOS;
    final theme = Theme.of(context);

    final stack = Stack(
      fit: StackFit.loose,
      children: [
        AnimatedBuilder(
          animation: cardEdit.controller,
          builder: (context, child) {
            final c = cardEdit.controller;

            return Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                const Spacer(),
                IconButton(
                  onPressed: c.canUndo ? () => c.undo() : null,
                  tooltip: 'Undo',
                  icon: const Icon(Icons.undo),
                ),
                const SizedBox(width: 4),
                IconButton(
                  onPressed: c.canRedo ? () => c.redo() : null,
                  tooltip: 'Redo',
                  icon: const Icon(Icons.redo),
                ),
                const Spacer(),
              ],
            );
          },
        ),
        Positioned(
          top: 0,
          left: 0,
          child: Row(
            children: [
              IconButton(
                onPressed: () {
                  _showBottomSheet(
                    context,
                    cardEdit: cardEdit,
                    accEdit: accEdit,
                    onPickFile: onPickFile,
                  );
                },
                tooltip: 'Add content',
                icon: const Icon(Icons.add_box),
              ),
              const SizedBox(width: 4),
              FormatTextButton(onFormatToolbar: onFormatToolbar),
            ],
          ),
        ),
      ],
    );

    // We want to see the bar when virtual keyboard is opened
    final bottomInset = MediaQuery.of(context).viewInsets.bottom;
    return Material(
      elevation: 3,
      color: theme.colorScheme.surface,
      surfaceTintColor: theme.colorScheme.surfaceTint,
      child: Padding(
        padding: EdgeInsets.only(
          bottom: dense ? bottomInset : bottomInset + 4,
          top: dense ? 0 : 4,
          left: 8,
          right: 8,
        ),
        child: SafeArea(child: stack),
      ),
    );
  }
}

void _showBottomSheet(BuildContext context,
    {required TimelineCardEdit cardEdit,
    required AccEdit accEdit,
    required void Function(FileType) onPickFile}) {
  showModalBottomSheet(
      context: context,
      builder: (BuildContext context) {
        return Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            ListTile(
              onTap: () {
                onPickFile(FileType.image);
                Navigator.pop(context);
              },
              leading: const Icon(Icons.image),
              title: const Text('Add image'),
            ),
            ListTile(
              onTap: () {
                onPickFile(FileType.any);
                Navigator.pop(context);
              },
              leading: const Icon(Icons.attach_file),
              title: const Text('Add file'),
            ),
            ListTile(
              onTap: () {
                cardEdit.addTask();
                Navigator.pop(context);
              },
              leading: const Icon(Icons.add_task),
              title: const Text('Add task'),
            ),
            ListTile(
              onTap: () {
                Navigator.pop(context);
                showLabelsPicker(context, accEdit: accEdit, cardEdit: cardEdit);
              },
              leading: const Icon(Icons.new_label),
              title: const Text('Add label'),
            ),
            // TextButton.icon(
            //   onPressed: () {},
            //   icon: const Icon(Icons.table_chart),
            //   label: const Text('Add table'),
            // ),
            // TextButton.icon(
            //   onPressed: () {},
            //   icon: const Icon(Icons.place),
            //   label: const Text('Add location'),
            // ),
            const SizedBox(height: 32),
          ],
        );
      });
}

void showLabelsPicker(BuildContext context,
    {required TimelineCardEdit cardEdit, required AccEdit accEdit}) {
  showDialogNav(
    context: context,
    initialRoute: BolikRoutes.cardLabels,
    builder: (context, child) => MultiProvider(
      providers: [
        ChangeNotifierProvider.value(value: accEdit),
        ChangeNotifierProvider.value(value: cardEdit),
      ],
      child: child,
    ),
  );
}
