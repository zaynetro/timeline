import 'dart:io';

import 'package:flutter/material.dart';
import 'package:path/path.dart' as p;
import 'package:provider/provider.dart';
import 'package:timeline/bridge_generated.dart';
import 'package:timeline/features/card/card_edit.dart';
import 'package:timeline/features/card/media_page.dart';

class CardImagePreview extends StatefulWidget {
  final CardFile file;
  final double columnWidth;

  const CardImagePreview(
      {super.key, required this.file, required this.columnWidth});

  @override
  State<StatefulWidget> createState() => _CardImagePreviewState();
}

class _CardImagePreviewState extends State<CardImagePreview> {
  double _imageScale = 1.0;

  @override
  void initState() {
    super.initState();
    _loadFile();
  }

  Future<void> _loadFile() async {
    final cardBlobs = context.read<CardBlobs>();
    cardBlobs.downloadFile(widget.file);
  }

  @override
  Widget build(BuildContext context) {
    final blob = context.select<CardBlobs, MaybeBlobPath?>(
        (value) => value.blobs[widget.file.blobId]);
    final cardEdit = context.read<TimelineCardEdit>();

    ImageProvider<Object>? provider;
    if (blob?.path != null) {
      // Display original file
      final width = widget.columnWidth * 2.2;
      provider =
          ResizeImage(FileImage(File(blob!.path!)), width: width.toInt());
    }

    if (provider == null) {
      return Container(
        color: Colors.grey[200],
        child: Stack(
          fit: StackFit.expand,
          alignment: Alignment.center,
          children: [
            Icon(Icons.image, size: 32, color: Colors.grey[800]),
            const Positioned(
              top: 16,
              right: 16,
              child: SizedBox(
                width: 20,
                height: 20,
                child: CircularProgressIndicator(strokeWidth: 2),
              ),
            ),
          ],
        ),
      );
    }

    final effectiveImageScale = _imageScale;
    return InkWell(
      onHover: (hovered) {
        setState(() => _imageScale = hovered ? 1.2 : 1.0);
      },
      onTapUp: (details) {
        // Add this handler to prevent tap being forwared to the editor
      },
      onTap: () async {
        // Remove focus from the editor
        // FocusManager.instance.primaryFocus?.unfocus();

        final cardBlobs = context.read<CardBlobs>();

        if (blob?.path != null) {
          final provider = FileImage(File(blob!.path!));
          await precacheImage(provider, context);
        }

        Navigator.push(
            context,
            MaterialPageRoute(
              builder: (context) => MultiProvider(
                providers: [
                  ChangeNotifierProvider.value(value: cardEdit),
                  ChangeNotifierProvider.value(value: cardBlobs),
                ],
                child: MediaPage(selectedBlobId: widget.file.blobId),
              ),
            ));

        // Remove focus from the editor
        // FocusManager.instance.primaryFocus?.unfocus();
      },
      child: Stack(
        fit: StackFit.expand,
        children: [
          Container(
            clipBehavior: Clip.hardEdge,
            decoration: const BoxDecoration(),
            child: AnimatedScale(
              scale: effectiveImageScale,
              duration: const Duration(milliseconds: 700),
              child: Hero(
                tag: widget.file.blobId,
                child: Image(
                  image: provider,
                  height: widget.columnWidth,
                  semanticLabel: widget.file.name,
                  fit: BoxFit.cover,
                  gaplessPlayback: true,
                ),
              ),
            ),
          ),
        ],
      ),
    );
  }
}

bool isImageFile(String? name) {
  if (name == null) return false;
  final ext = p.extension(name).toLowerCase();
  final supported = ['.jpg', '.jpeg', '.gif', '.png', '.webp'];
  if (!Platform.isLinux) {
    supported.add('.heic');
  }

  return supported.contains(ext);
}
