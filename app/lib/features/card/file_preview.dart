import 'dart:io';

import 'package:flutter/material.dart';
import 'package:open_filex/open_filex.dart';
import 'package:provider/provider.dart';
import 'package:timeline/bridge_generated.dart';
import 'package:timeline/common/models/app_state.dart';
import 'package:timeline/features/card/card_edit.dart';
import 'package:url_launcher/url_launcher.dart';

class CardFilePreview extends StatefulWidget {
  final CardFile file;

  const CardFilePreview(this.file, {Key? key}) : super(key: key);

  @override
  State<StatefulWidget> createState() => _FilePreviewState();
}

class _FilePreviewState extends State<CardFilePreview> {
  bool opening = false;
  bool _shouldOpen = false;

  _openFile(String filePath) {
    _shouldOpen = false;
    if (!opening) {
      setState(() => opening = true);
      Future.delayed(const Duration(seconds: 5)).then((_) {
        if (mounted) {
          setState(() => opening = false);
        }
      });

      if (Platform.isAndroid || Platform.isIOS) {
        OpenFilex.open(filePath, linuxByProcess: true);
      } else {
        launchUrl(Uri.parse('file:$filePath'));
      }
    }
  }

  @override
  Widget build(BuildContext context) {
    final blob = context.select<CardBlobs, MaybeBlobPath?>(
        (value) => value.blobs[widget.file.blobId]);
    final cardEdit = context.read<TimelineCardEdit>();
    final accEdit = context.read<AccEdit>();
    final readonly = cardEdit.readonly(accEdit.view);

    Widget icon = const Icon(Icons.attachment);
    final downloading = blob?.downloading == true;
    if (downloading) {
      icon = const CircularProgressIndicator();
    } else if (opening) {
      icon = const Icon(Icons.hourglass_empty);
    }

    // File is downloaded --> open
    if (blob?.path != null && _shouldOpen) {
      WidgetsBinding.instance.addPostFrameCallback((_) {
        _openFile(blob!.path!);
      });
    }

    final card = Card(
      child: ListTile(
        onTap: () {
          // Remove focus from the editor
          // FocusManager.instance.primaryFocus?.unfocus();

          if (downloading || opening) {
            return;
          }

          // We have the path --> open
          if (blob?.path != null) {
            _openFile(blob!.path!);
            return;
          }

          // Download the file and open it after
          final cardBlobs = context.read<CardBlobs>();
          _shouldOpen = true;
          cardBlobs.downloadFile(widget.file);
        },
        leading: icon,
        title: Text(widget.file.name ?? 'No name'),
        subtitle: Text(_fileSizeDisplay(widget.file.sizeBytes)),
        trailing: readonly
            ? null
            : PopupMenuButton<String>(
                tooltip: 'Options',
                onSelected: (action) {
                  final cardEdit = context.read<TimelineCardEdit>();
                  if (action == 'delete') {
                    cardEdit.removeFile(widget.file);
                  }
                },
                itemBuilder: (context) => const [
                  PopupMenuItem(value: 'delete', child: Text('Delete')),
                ],
              ),
      ),
    );

    return GestureDetector(
      onTapUp: (details) {
        // Add this handler to prevent tap being forwared to the editor
      },
      child: card,
    );
  }
}

String _fileSizeDisplay(int sizeBytes) {
  const divider = 1024;
  if (sizeBytes < divider) {
    return '$sizeBytes B';
  }

  final kBytes = sizeBytes / divider;
  if (kBytes < divider) {
    return '${kBytes.toStringAsFixed(1)} KB';
  }

  final mBytes = kBytes / divider;
  if (mBytes < divider) {
    return '${mBytes.toStringAsFixed(1)} MB';
  }

  final gBytes = kBytes / divider;
  return '${gBytes.toStringAsFixed(1)} GB';
}
