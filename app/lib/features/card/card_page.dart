import 'dart:math';

import 'package:file_picker/file_picker.dart';
import 'package:fleather/fleather.dart';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:timeline/bridge_ext.dart';
import 'package:timeline/bridge_generated.dart';

import 'package:timeline/common/models/app_state.dart';
import 'package:timeline/common/models/phase_info.dart';
import 'package:timeline/features/card/block_row.dart';
import 'package:timeline/features/card/bottom_bar.dart';
import 'package:timeline/features/card/card_edit.dart';
import 'package:timeline/features/card/file_preview.dart';
import 'package:timeline/features/card/image_preview.dart';
import 'package:timeline/features/card/move_to_bin.dart';
import 'package:timeline/routes.dart';
import 'package:url_launcher/url_launcher.dart';

class CardPageArguments {
  final CardView card;
  final StartTemplate? template;
  final List<String> selectedLabelIds;

  CardPageArguments(this.card, {this.template, required this.selectedLabelIds});
}

enum StartTemplate {
  pick,
  note,
  media,
  tasks,
}

class CardPage extends StatelessWidget {
  const CardPage({Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    final args =
        ModalRoute.of(context)!.settings.arguments as CardPageArguments;

    final state = context.read<AppState>();

    return MultiProvider(
        providers: [
          ChangeNotifierProvider(
              create: (_) =>
                  TimelineCardEdit(args.card, state.dispatcher, state.native)),
          ChangeNotifierProvider(
              create: (_) =>
                  CardBlobs(args.card.id, state.dispatcher, state.native)),
          ChangeNotifierProvider(create: (_) => CardDragState()),
        ],
        child: _CardBody(
          startTemplate: args.template,
          startSelectedLabelIds: args.selectedLabelIds,
        ));
  }
}

class _CardBody extends StatefulWidget {
  final StartTemplate? startTemplate;
  final List<String> startSelectedLabelIds;

  const _CardBody(
      {Key? key, this.startTemplate, required this.startSelectedLabelIds})
      : super(key: key);

  @override
  State<StatefulWidget> createState() => _CardBodyState();
}

class _CardBodyState extends State<_CardBody> with WidgetsBindingObserver {
  final scaffoldKey = GlobalKey<ScaffoldState>();
  var formatToolbarOpen = false;

  // Cache context values so that we can safely use them in dispose method
  late TimelineCardEdit cardEdit;
  late AppState appState;
  late AccEdit accEdit;

  @override
  void initState() {
    super.initState();
    WidgetsBinding.instance.addObserver(this);

    cardEdit = context.read<TimelineCardEdit>();
    appState = context.read<AppState>();
    accEdit = context.read<AccEdit>();

    _initTemplate();
  }

  void _initTemplate() async {
    final cardEdit = context.read<TimelineCardEdit>();
    for (var labelId in widget.startSelectedLabelIds) {
      if (labelId == deletedLabelId) {
        continue;
      }

      await cardEdit.addLabel(labelId);
    }
  }

  @override
  void dispose() {
    WidgetsBinding.instance.removeObserver(this);
    saveDoc();
    super.dispose();
  }

  @override
  void didChangeAppLifecycleState(AppLifecycleState state) {
    if (state == AppLifecycleState.paused) {
      cardEdit.queueSave();
    }
  }

  Future<void> saveDoc() async {
    if (cardEdit.readonly(accEdit.view)) {
      return;
    }

    await cardEdit.close();
    if (cardEdit.isEmpty) {
      await cardEdit.moveToBin();

      if (!mounted) return;
      ScaffoldMessenger.of(context).showSnackBar(
          const SnackBar(content: Text('Empty card moved to bin')));
    }

    appState.timeline.refresh();
  }

  _onPickFile(FileType fileType) async {
    final cardEdit = context.read<TimelineCardEdit>();

    // Ref: https://github.com/miguelpruivo/flutter_file_picker/wiki/API#-filepickerpickfiles
    FilePickerResult? result = await FilePicker.platform.pickFiles(
      allowMultiple: true,
      type: fileType,
    );

    const maxFileSize = 15 * 1024 * 1024;

    if (result != null) {
      for (var file in result.files) {
        if (file.size > maxFileSize) {
          continue;
        }

        if (file.path != null) {
          await _addAttachment(file.path!);
        }
      }

      cardEdit.queueSave();
    }
  }

  _addAttachment(String path) async {
    logger.debug('Selected file: path=$path');

    final cardEdit = context.read<TimelineCardEdit>();
    try {
      await cardEdit.attachFile(path);
    } catch (e) {
      logger.warn('Failed to attach a file: $e');
    }
  }

  @override
  Widget build(BuildContext context) {
    final accEdit = context.read<AccEdit>();
    final cardEdit = context.read<TimelineCardEdit>();
    final readOnly = cardEdit.readonly(accEdit.view);
    final inBin = cardEdit.inBin();
    final theme = Theme.of(context);

    return Scaffold(
      key: scaffoldKey,
      appBar: AppBar(
        actions: [
          PopupMenuButton<_PopupActions>(
            tooltip: 'Options',
            onSelected: (action) async {
              if (action == _PopupActions.delete) {
                _showMoveToBinDialog(context,
                    accEdit: accEdit, cardEdit: cardEdit);
              } else if (action == _PopupActions.share) {
                // Edit collaborators
                showDialogNav(
                  context: context,
                  initialRoute: BolikRoutes.cardCollaborators,
                  builder: (context, child) => MultiProvider(
                    providers: [
                      ChangeNotifierProvider.value(value: accEdit),
                      ChangeNotifierProvider.value(value: cardEdit),
                    ],
                    child: child,
                  ),
                );
              } else if (action == _PopupActions.restore) {
                final cardEdit = context.read<TimelineCardEdit>();
                final card = await cardEdit.restore();
                if (mounted) {
                  Navigator.pushReplacementNamed(context, BolikRoutes.card,
                      arguments: CardPageArguments(card, selectedLabelIds: []));
                }
              }
            },
            itemBuilder: (context) => [
              if (!inBin)
                PopupMenuItem(
                  value: _PopupActions.share,
                  child: Row(children: const [
                    Icon(Icons.groups),
                    SizedBox(width: 16),
                    Text('Collaborators')
                  ]),
                ),
              if (!inBin)
                PopupMenuItem(
                  value: _PopupActions.delete,
                  child: Row(children: [
                    Icon(Icons.delete, color: theme.colorScheme.error),
                    const SizedBox(width: 16),
                    Text('Delete',
                        style: TextStyle(color: theme.colorScheme.error))
                  ]),
                ),
              if (inBin)
                PopupMenuItem(
                  value: _PopupActions.restore,
                  child: Row(children: const [
                    Icon(Icons.restore_from_trash),
                    SizedBox(width: 16),
                    Text('Restore a copy')
                  ]),
                ),
            ],
          ),
          TextButton.icon(
            onPressed: () {
              Navigator.pop(context);
            },
            icon: const Icon(Icons.done),
            label: const Text('Done'),
          ),
        ],
      ),
      body: Padding(
        padding: EdgeInsets.only(bottom: formatToolbarOpen ? 60 : 0),
        child: const CardContent(),
      ),
      bottomNavigationBar: readOnly
          ? null
          : CardBottomBar(
              onPickFile: _onPickFile,
              onFormatToolbar: (isOpen) {
                if (mounted) {
                  setState(() => formatToolbarOpen = isOpen);
                }
              },
            ),
    );
  }
}

class CardContent extends StatefulWidget {
  const CardContent({Key? key}) : super(key: key);

  @override
  State<StatefulWidget> createState() => CardContentState();
}

class CardContentState extends State<CardContent> {
  final _scrollKey = GlobalKey();
  final _scrollController = ScrollController();
  late BoxConstraints _layoutConstraints;
  static const maxContentWidth = 900.0;

  @override
  void dispose() {
    _scrollController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final cardEdit = context.read<TimelineCardEdit>();
    final accEdit = context.read<AccEdit>();
    final readOnly = cardEdit.readonly(accEdit.view);
    // final today = DateTime.now();
    // final editedAtLocal = cardEdit.card.editedAt.toLocal();
    // final daysDiff = today.difference(editedAtLocal).inDays;
    // // If edited today then display time. Otherwise display the day.
    // final editedAt = daysDiff == 0
    //     ? DateFormat.Hm().format(editedAtLocal)
    //     : DateFormat.MMMd().format(editedAtLocal);

    // Draw content
    final rows = <Widget>[];

    // TODO: here display when document was created and who it is shared with

    rows.add(LayoutBuilder(builder: (context, constraints) {
      _layoutConstraints = constraints;
      return FleatherEditor(
        readOnly: readOnly,
        showCursor: !readOnly,
        scrollable: false,
        autofocus: cardEdit.isEmpty,
        scrollController: _scrollController,
        controller: cardEdit.controller,
        padding: const EdgeInsets.symmetric(vertical: 8, horizontal: 16),
        maxContentWidth: maxContentWidth,
        onLaunchUrl: (url) {
          if (url != null) {
            launchUrl(Uri.parse(url), mode: LaunchMode.externalApplication);
          }
        },
        embedBuilder: _embedBuilder,
      );
    }));

    rows.add(const SizedBox(height: 10));
    rows.add(
        CardBlockRow(maxContentWidth: maxContentWidth, child: _LabelsRow()));

    return SingleChildScrollView(
      key: _scrollKey,
      controller: _scrollController,
      scrollDirection: Axis.vertical,
      child: Column(
        children: rows,
      ),
    );
  }

  Widget _embedBuilder(BuildContext context, EmbedNode node) {
    if (node.value.type == 'bolik-file') {
      final maxWidth = min(_layoutConstraints.maxWidth, maxContentWidth);
      const totalPadding = 32.0;

      final file = node.value.data['obj'] as CardFile;
      if (isImageFile(file.name)) {
        const maxImageWidth = 250;
        final columns = (maxWidth / maxImageWidth).ceil();
        final columnWidth =
            ((maxWidth - totalPadding) / columns).floor().toDouble();

        return _FileEmbedWidget(
          width: columnWidth,
          child: CardImagePreview(file: file, columnWidth: columnWidth),
        );
      }

      final columns = maxWidth < 700 ? 1 : 2;
      final columnWidth =
          ((maxWidth - totalPadding) / columns).floor().toDouble();
      return _FileEmbedWidget(
        width: columnWidth,
        child: CardFilePreview(file),
      );
    }

    return const Text('?');
  }
}

class _LabelsRow extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    final accEdit = context.read<AccEdit>();
    final cardEdit = context.watch<TimelineCardEdit>();
    final readonly = cardEdit.readonly(accEdit.view);

    return Wrap(
      direction: Axis.horizontal,
      spacing: 8,
      runSpacing: 8,
      children: cardEdit.card
          .accLabels(accEdit.view)
          .map((label) => InputChip(
                label: Text(label.name),
                onPressed: readonly
                    ? null
                    : () {
                        showLabelsPicker(context,
                            accEdit: accEdit, cardEdit: cardEdit);
                      },
              ))
          .toList(),
    );
  }
}

enum _PopupActions {
  share,
  delete,
  restore,
}

void _showMoveToBinDialog(BuildContext context,
    {required TimelineCardEdit cardEdit, required AccEdit accEdit}) {
  showDialog(
    context: context,
    builder: (context) => MultiProvider(providers: [
      ChangeNotifierProvider.value(value: accEdit),
      ChangeNotifierProvider.value(value: cardEdit),
    ], child: const MoveToBinDialog()),
  );
}

class _FileEmbedWidget extends StatelessWidget {
  final Widget child;
  final double width;

  const _FileEmbedWidget({super.key, required this.width, required this.child});

  @override
  Widget build(BuildContext context) {
    return Container(
      padding: const EdgeInsets.only(left: 4, top: 2, right: 2, bottom: 2),
      constraints:
          BoxConstraints(minWidth: width, maxWidth: width, maxHeight: width),
      child: child,
    );
  }
}
