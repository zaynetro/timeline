import 'package:flutter/material.dart';
import 'package:timeline/bridge_generated.dart';
import 'package:timeline/features/card/card_page.dart';
import 'package:timeline/routes.dart';

const double cardPreviewMaxHeight = 250;

class CardPreviewStack extends StatefulWidget {
  final CardView card;

  const CardPreviewStack(this.card, {super.key});

  @override
  State<StatefulWidget> createState() => _CardPreviewStackState();
}

class _CardPreviewStackState extends State<CardPreviewStack> {
  final animationDuration = const Duration(milliseconds: 200);
  var hovered = false;

  void _onTap() async {
    Navigator.pushNamed(context, BolikRoutes.card,
        arguments: CardPageArguments(widget.card, selectedLabelIds: []));
  }

  @override
  Widget build(BuildContext context) {
    final preview = _PreviewCard.from(widget.card);
    final colorScheme = Theme.of(context).colorScheme;

    return InkWell(
      onTap: _onTap,
      onHover: (value) {
        setState(() => hovered = value);
      },
      child: Container(
        // Set border when there is no image
        decoration: preview.image != null
            ? null
            : BoxDecoration(
                border: Border.all(color: colorScheme.inversePrimary),
                color: colorScheme.primaryContainer.withOpacity(0.1),
              ),
        child: LayoutBuilder(builder: (context, constraints) {
          if (preview.image != null) {
            return _CardImageTile(
              image: preview.image!,
              preview: preview,
              hovered: hovered,
              maxHeight: constraints.maxHeight,
            );
          }

          return _CardTile(
            hovered: hovered,
            preview: preview,
            maxHeight: constraints.maxHeight,
          );
        }),
      ),
    );
  }
}

class _CardShortStats extends StatelessWidget {
  final _PreviewCard preview;
  final bool hasImage;

  const _CardShortStats({required this.preview, required this.hasImage});

  @override
  Widget build(BuildContext context) {
    final rows = <Widget>[];

    final textStyle = TextStyle(fontSize: 12, color: Colors.grey[800]);
    const iconSize = 16.0;

    // Short tasks
    final totalTasks = preview.allTasks.total;
    if (totalTasks > 0) {
      final completed = preview.allTasks.completed.length;
      rows.add(Row(children: [
        const Icon(Icons.check, size: iconSize),
        const SizedBox(width: 2),
        Text('$completed / $totalTasks', style: textStyle),
      ]));
    }

    // Short files
    if (preview.allFiles.isNotEmpty) {
      if (rows.isNotEmpty) {
        rows.add(VerticalDivider(
          width: 16,
          thickness: 1,
          color: Colors.grey.withOpacity(0.4),
        ));
      }

      rows.add(Row(children: [
        const Icon(Icons.attachment, size: iconSize),
        const SizedBox(width: 2),
        Text('${preview.allFiles.length}', style: textStyle),
      ]));
    }

    if (rows.isEmpty) {
      return const SizedBox();
    }

    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 8),
      decoration: BoxDecoration(
        borderRadius: const BorderRadius.all(Radius.circular(5)),
        color: Colors.grey[100]!.withOpacity(0.8),
        boxShadow: [
          if (hasImage)
            const BoxShadow(
              color: Colors.black38,
              blurRadius: 8,
            ),
        ],
      ),
      height: 30,
      child: Row(children: rows),
    );
  }
}

class _CardImageTile extends StatelessWidget {
  final _PreviewImage image;
  final bool hovered;
  final _PreviewCard preview;
  final double maxHeight;

  const _CardImageTile({
    super.key,
    required this.image,
    required this.hovered,
    required this.preview,
    required this.maxHeight,
  });

  @override
  Widget build(BuildContext context) {
    final imageWidget = Container(
      clipBehavior: Clip.antiAlias,
      decoration: const BoxDecoration(),
      child: AnimatedScale(
        scale: hovered ? 1.2 : 1.0,
        duration: const Duration(milliseconds: 700),
        child: Image(
          image: image.provider,
          gaplessPlayback: true,
          fit: BoxFit.cover,
        ),
      ),
    );

    Widget? card;
    if (preview.lines.isNotEmpty) {
      card = _CardContainer(
        hovered: hovered,
        child: ScrollConfiguration(
          behavior: ScrollConfiguration.of(context).copyWith(scrollbars: false),
          child: SingleChildScrollView(
            physics: const NeverScrollableScrollPhysics(),
            child: Column(
              mainAxisAlignment: MainAxisAlignment.center,
              crossAxisAlignment: CrossAxisAlignment.start,
              children: preview.lines,
            ),
          ),
        ),
      );
    }

    return Stack(
      fit: StackFit.expand,
      children: [
        Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            // Image
            Expanded(flex: 2, child: imageWidget),

            if (card != null) Flexible(flex: 1, child: card),
          ],
        ),

        // Short files/tasks
        if (preview.hasFiles || preview.hasTasks)
          Positioned(
            top: 8,
            right: 8,
            child: _CardShortStats(preview: preview, hasImage: true),
          )
      ],
    );
  }
}

class _CardTile extends StatelessWidget {
  final bool hovered;
  final _PreviewCard preview;
  final double maxHeight;

  const _CardTile({
    super.key,
    required this.hovered,
    required this.preview,
    required this.maxHeight,
  });

  @override
  Widget build(BuildContext context) {
    final hasShortStats = preview.hasTasks || preview.hasFiles;

    final heightForBlocks = hasShortStats ? maxHeight - 30 : maxHeight;
    final heightPerBlock = heightForBlocks / 2;
    final maxRowsPerBlock = (heightPerBlock / 32).floor();

    Widget? files;

    // Files
    if (preview.lines.isEmpty) {
      final all = preview.allFiles;
      files = Column(
        children: all
            .where((f) => f.isNotEmpty)
            .take(maxRowsPerBlock)
            .map((fileName) => Padding(
                  padding: const EdgeInsets.symmetric(vertical: 2),
                  child: Row(
                    children: [
                      const Icon(Icons.attachment),
                      const SizedBox(width: 8),
                      Flexible(
                        child: Text(fileName, overflow: TextOverflow.ellipsis),
                      ),
                    ],
                  ),
                ))
            .toList(),
      );
    }

    // Fallback
    if (preview.lines.isEmpty && !hasShortStats) {
      preview.lines.add(const Text('Empty'));
    }

    final text = Padding(
      padding: const EdgeInsets.symmetric(vertical: 4),
      child: ScrollConfiguration(
        behavior: ScrollConfiguration.of(context).copyWith(scrollbars: false),
        child: SingleChildScrollView(
          physics: const NeverScrollableScrollPhysics(),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            children: preview.lines,
          ),
        ),
      ),
    );

    return _CardContainer(
      hovered: hovered,
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          // Short stats
          if (hasShortStats)
            Row(
              children: [
                const Spacer(),
                FittedBox(
                  child: _CardShortStats(preview: preview, hasImage: false),
                ),
              ],
            ),

          if (preview.lines.isNotEmpty) Flexible(child: text),
          if (files != null) Expanded(child: files),
        ],
      ),
    );
  }
}

class _CardContainer extends StatelessWidget {
  final bool hovered;
  final Widget child;

  const _CardContainer({required this.hovered, required this.child});

  @override
  Widget build(BuildContext context) {
    final colorScheme = Theme.of(context).colorScheme;
    final opacityBounds = [0.5, 0.9];

    return AnimatedContainer(
      duration: const Duration(milliseconds: 200),
      decoration: BoxDecoration(
        color: hovered
            ? colorScheme.primaryContainer.withOpacity(opacityBounds[1])
            : colorScheme.primaryContainer.withOpacity(opacityBounds[0]),
      ),
      alignment: Alignment.center,
      padding: const EdgeInsets.all(8),
      clipBehavior: Clip.antiAlias,
      child: child,
    );
  }
}

class _PreviewCard {
  final _PreviewImage? image;
  final List<Widget> lines;
  final List<String> allFiles;
  final _PreviewTasks allTasks;

  _PreviewCard(this.image, this.lines, this.allFiles, this.allTasks);

  bool get hasTasks => allTasks.total > 0;
  bool get hasFiles => allFiles.isNotEmpty;

  static _PreviewCard from(CardView card) {
    _PreviewImage? image;
    List<_PreviewTask> completedTasks = [];
    List<_PreviewTask> pendingTasks = [];
    List<String> allFiles = [];

    String previousBlock = '';
    String currentLine = '';
    List<Widget> lines = [];
    var onlyEmptyLines = true;

    if (card.thumbnail != null) {
      image = _PreviewImage(
        card.id,
        MemoryImage(card.thumbnail!.data),
      );
    }

    const maxLines = 20;

    void appendCurrent({String block = '', TextStyle? style}) {
      if (lines.length < maxLines) {
        // Skip empty lines in the beginning of the card
        if (currentLine.isEmpty && lines.isEmpty) {
          currentLine = '';
          return;
        }

        if (previousBlock != block) {
          lines.add(const SizedBox(height: 4));
          previousBlock = block;
        }

        // We want to know if we add only empty text lines
        if (onlyEmptyLines && currentLine.isNotEmpty) {
          onlyEmptyLines = false;
        }

        lines.add(Text(currentLine, style: style));
      }

      currentLine = '';
    }

    void appendWidget(Widget w, {String block = ''}) {
      if (lines.length < 20) {
        if (previousBlock != block) {
          lines.add(const SizedBox(height: 4));
          previousBlock = block;
        }

        onlyEmptyLines = false;
        lines.add(w);
      }
      currentLine = '';
    }

    for (final block in card.blocks) {
      final contentView = block.view;
      if (contentView is ContentView_Text) {
        final attrs = contentView.field0.attrs;

        if (attrs?.heading != null) {
          appendCurrent(
            style: const TextStyle(fontSize: 17),
            block: 'h',
          );
        } else if (attrs?.block == 'cl') {
          final completed = attrs?.checked == true;
          if (completed) {
            completedTasks.add(_PreviewTask(true, currentLine));
          } else {
            pendingTasks.add(_PreviewTask(false, currentLine));
          }

          appendWidget(
            Row(
              children: [
                if (completed) const Icon(Icons.check_box),
                if (!completed) const Icon(Icons.check_box_outline_blank),
                const SizedBox(width: 8),
                Flexible(
                  child: Text(currentLine, overflow: TextOverflow.ellipsis),
                ),
              ],
            ),
            block: 'cl',
          );
        } else if (attrs?.block == 'ul') {
          appendWidget(Text('  • $currentLine'), block: 'ul');
        } else if (attrs?.block == 'ol') {
          appendWidget(Text('  • $currentLine'), block: 'ol');
        } else {
          final blockLines = contentView.field0.value.split('\n');
          for (var i = 0; i < blockLines.length; i += 1) {
            final line = blockLines[i];
            if (i == (blockLines.length - 1)) {
              currentLine += line;
            } else {
              currentLine += line;
              appendCurrent();
            }
          }
        }
      } else if (contentView is ContentView_File) {
        final fileName = contentView.field0.name;
        allFiles.add(fileName ?? '');
      }
    }

    if (currentLine.isNotEmpty) {
      appendCurrent();
    }

    return _PreviewCard(
      image,
      onlyEmptyLines ? [] : lines,
      allFiles,
      _PreviewTasks(completedTasks, pendingTasks),
    );
  }
}

class _PreviewImage {
  final String blobId;
  final ImageProvider provider;

  _PreviewImage(this.blobId, this.provider);
}

class _PreviewTasks {
  final List<_PreviewTask> completed;
  final List<_PreviewTask> pending;

  _PreviewTasks(this.completed, this.pending);

  int get total => completed.length + pending.length;
}

class _PreviewTask {
  final bool completed;
  final String text;

  _PreviewTask(this.completed, this.text);
}
