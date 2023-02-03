import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:provider/provider.dart';
import 'package:timeline/bridge_generated.dart';
import 'package:timeline/common/models/app_state.dart';
import 'package:timeline/features/card/card_edit.dart';
import 'package:timeline/features/card/image_preview.dart';
import 'package:url_launcher/url_launcher.dart';
import 'package:open_filex/open_filex.dart';

// References:
// * https://github.com/gskinnerTeam/flutter-wonderous-app/blob/master/lib/ui/common/modals/fullscreen_url_img_viewer.dart
// * https://github.com/qq326646683/interactiveviewer_gallery/blob/main/lib/interactiveviewer_gallery.dart

class MediaPage extends StatefulWidget {
  final String selectedBlobId;

  const MediaPage({Key? key, required this.selectedBlobId}) : super(key: key);

  @override
  State<StatefulWidget> createState() => _MediaPageState();
}

class _MediaPageState extends State<MediaPage> {
  late PageController pageController;
  late final List<_MediaFileDoc> mediae;

  @override
  void initState() {
    super.initState();

    final cardEdit = context.read<TimelineCardEdit>();
    mediae = _mediaFiles(cardEdit);
    final initialPage =
        mediae.indexWhere((doc) => doc.file.blobId == widget.selectedBlobId);

    pageController = PageController(initialPage: initialPage);
  }

  @override
  void dispose() {
    pageController.dispose();
    super.dispose();
  }

  List<_MediaFileDoc> _mediaFiles(TimelineCardEdit cardEdit) {
    final mediae = <_MediaFileDoc>[];
    for (var block in cardEdit.card.blocks) {
      final contentView = block.view;
      if (contentView is ContentView_File) {
        final file = contentView.field0;
        if (isImageFile(file.name)) {
          mediae.add(_MediaFileDoc(block.position, file));
        }
      }
    }
    return mediae;
  }

  @override
  Widget build(BuildContext context) {
    final cardEdit = context.read<TimelineCardEdit>();
    final accEdit = context.read<AccEdit>();
    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(
        title: ChangeNotifierProvider.value(
          value: pageController,
          child: _PageHeader(pagesCount: mediae.length),
        ),
        actions: [
          PopupMenuButton<_ExtraActions>(
            tooltip: 'Options',
            onSelected: (action) {
              final page = pageController.page?.round();
              if (page != null) {
                if (action == _ExtraActions.openIn) {
                  final cardBlobs = context.read<CardBlobs>();
                  final blobId = mediae[page].file.blobId;
                  final blob = cardBlobs.blobs[blobId];
                  if (blob?.path == null) {
                    return;
                  }

                  if (Platform.isAndroid || Platform.isIOS) {
                    OpenFilex.open(blob!.path!, linuxByProcess: true);
                  } else {
                    launchUrl(Uri.parse('file:${blob!.path!}'));
                  }
                } else if (action == _ExtraActions.delete) {
                  final file = mediae[page].file;
                  cardEdit.removeFile(file);
                  Navigator.pop(context);
                }
              }
            },
            itemBuilder: (context) => [
              PopupMenuItem(
                value: _ExtraActions.openIn,
                child: Row(children: const [
                  Icon(Icons.open_in_new),
                  SizedBox(width: 16),
                  Text('Open In'),
                ]),
              ),
              if (!cardEdit.readonly(accEdit.view))
                PopupMenuItem(
                  value: _ExtraActions.delete,
                  child: Row(children: [
                    Icon(Icons.delete, color: theme.colorScheme.error),
                    const SizedBox(width: 16),
                    Text('Delete',
                        style: TextStyle(color: theme.colorScheme.error))
                  ]),
                ),
            ],
          ),
        ],
      ),
      body: _MediaContent(
        mediae: mediae,
        pageController: pageController,
      ),
    );
  }
}

class _ZoomStatus {
  final bool isZoomed;
  final bool twoPointersOrMore;

  _ZoomStatus({required this.isZoomed, required this.twoPointersOrMore});

  _ZoomStatus copyWith({bool? isZoomed, bool? twoPointersOrMore}) =>
      _ZoomStatus(
        isZoomed: isZoomed ?? this.isZoomed,
        twoPointersOrMore: twoPointersOrMore ?? this.twoPointersOrMore,
      );

  @override
  String toString() =>
      'ZoomStatus(isZoomed=$isZoomed, twoPointersOrMore=$twoPointersOrMore)';
}

const _animateToPageDur = Duration(milliseconds: 150);
const _animateToPageCurve = Curves.easeInOut;

// ignore: must_be_immutable
class _MediaContent extends StatelessWidget {
  final List<_MediaFileDoc> mediae;
  final PageController pageController;
  final _zoomStatus = ValueNotifier(_ZoomStatus(
    isZoomed: false,
    twoPointersOrMore: false,
  ));
  var _pointers = 0;
  final _focusNode = FocusNode();
  final _isDesktop = Platform.isLinux || Platform.isWindows || Platform.isMacOS;

  _MediaContent({
    Key? key,
    required this.mediae,
    required this.pageController,
  }) : super(key: key);

  void _animatePrevPage() {
    final page = pageController.page?.round() ?? 0;
    if (page > 0) {
      pageController.animateToPage(page - 1,
          duration: _animateToPageDur, curve: _animateToPageCurve);
    }
  }

  void _animateNextPage() {
    final page = pageController.page?.round() ?? 0;
    if (page < mediae.length - 1) {
      pageController.animateToPage(page + 1,
          duration: _animateToPageDur, curve: _animateToPageCurve);
    }
  }

  Widget _wrapKeyboard(Widget child) {
    if (_isDesktop) {
      return RawKeyboardListener(
        focusNode: _focusNode,
        onKey: (event) {
          if (event.logicalKey == LogicalKeyboardKey.arrowLeft) {
            _animatePrevPage();
          } else if (event.logicalKey == LogicalKeyboardKey.arrowRight) {
            _animateNextPage();
          }
        },
        child: child,
      );
    }

    return child;
  }

  Widget _wrapArrowButtons(
      {required BuildContext context, required Widget child}) {
    if (_isDesktop) {
      return Stack(
        fit: StackFit.expand,
        children: [
          child,
          Positioned(
            left: 8,
            top: 0,
            bottom: 0,
            child: _ArrowButton(
              onTap: _animatePrevPage,
              icon: Icons.navigate_before,
            ),
          ),
          Positioned(
            right: 8,
            top: 0,
            bottom: 0,
            child: _ArrowButton(
              onTap: _animateNextPage,
              icon: Icons.navigate_next,
            ),
          ),
        ],
      );
    }

    return child;
  }

  Widget _wrapListener(Widget child) {
    return Listener(
      onPointerDown: (_) {
        _pointers += 1;

        final oldTwoPointersOrMore = _zoomStatus.value.twoPointersOrMore;
        if (_pointers > 1 && !oldTwoPointersOrMore) {
          _zoomStatus.value =
              _zoomStatus.value.copyWith(twoPointersOrMore: true);
        }
      },
      onPointerUp: (_) {
        _pointers -= 1;

        final oldTwoPointersOrMore = _zoomStatus.value.twoPointersOrMore;
        if (_pointers == 1 && oldTwoPointersOrMore) {
          _zoomStatus.value =
              _zoomStatus.value.copyWith(twoPointersOrMore: false);
        }
      },
      child: child,
    );
  }

  @override
  Widget build(BuildContext context) {
    return _wrapKeyboard(
      _wrapArrowButtons(
        context: context,
        child: _wrapListener(
          AnimatedBuilder(
            animation: _zoomStatus,
            builder: (_, __) {
              final bool disableSwipe = _zoomStatus.value.isZoomed ||
                  _zoomStatus.value.twoPointersOrMore;

              return PageView.builder(
                physics: disableSwipe
                    ? const NeverScrollableScrollPhysics()
                    : const PageScrollPhysics(),
                // This setting sets cacheExtend to 1 which caches next and previous pages.
                // https://github.com/flutter/flutter/issues/31191#issuecomment-828176688
                allowImplicitScrolling: true,
                itemCount: mediae.length,
                controller: pageController,
                itemBuilder: (BuildContext context, int index) {
                  final media = mediae[index];
                  return _MediaViewer(media.file, _zoomStatus);
                },
              );
            },
          ),
        ),
      ),
    );
  }
}

class _MediaViewer extends StatefulWidget {
  final CardFile file;
  final ValueNotifier<_ZoomStatus> zoomStatus;

  const _MediaViewer(this.file, this.zoomStatus);

  @override
  State<StatefulWidget> createState() => _MediaViewerState();
}

class _MediaViewerState extends State<_MediaViewer>
    with SingleTickerProviderStateMixin {
  final _controller = TransformationController();
  late Offset _doubleTapLocalPosition;

  @override
  void dispose() {
    _controller.dispose();
    super.dispose();
  }

  /// Reset zoom level to 1 on double-tap
  void _handleDoubleTap() {
    final currentScale = _controller.value.getMaxScaleOnAxis();

    if (currentScale != 1.0) {
      // Reset zoom
      _controller.value = Matrix4.identity();
      widget.zoomStatus.value =
          widget.zoomStatus.value.copyWith(isZoomed: false);
    } else {
      // Zoom in
      var matrix = _controller.value.clone();
      const targetScale = 2.0;

      final offSetX = -_doubleTapLocalPosition.dx * (targetScale - 1);
      final offSetY = -_doubleTapLocalPosition.dy * (targetScale - 1);

      matrix = Matrix4.fromList([
        targetScale,
        matrix.row1.x,
        matrix.row2.x,
        matrix.row3.x,
        matrix.row0.y,
        targetScale,
        matrix.row2.y,
        matrix.row3.y,
        matrix.row0.z,
        matrix.row1.z,
        targetScale,
        matrix.row3.z,
        offSetX,
        offSetY,
        matrix.row2.w,
        matrix.row3.w
      ]);

      _controller.value = matrix;
      widget.zoomStatus.value =
          widget.zoomStatus.value.copyWith(isZoomed: true);
    }
  }

  @override
  Widget build(BuildContext context) {
    final blob = context.select<CardBlobs, MaybeBlobPath?>(
        (value) => value.blobs[widget.file.blobId]);

    ImageProvider<Object>? provider;
    if (blob?.path != null) {
      // Display original file
      provider = FileImage(File(blob!.path!));
    } else {
      return Text('File ${widget.file.name}');
    }

    return GestureDetector(
      onDoubleTapDown: (TapDownDetails details) {
        _doubleTapLocalPosition = details.localPosition;
      },
      onDoubleTap: _handleDoubleTap,
      child: InteractiveViewer(
        transformationController: _controller,
        onInteractionEnd: (details) {
          final isZoomed = _controller.value.getMaxScaleOnAxis() > 1;
          widget.zoomStatus.value =
              widget.zoomStatus.value.copyWith(isZoomed: isZoomed);
        },
        minScale: 1,
        maxScale: 5,
        child: Center(
          child: Stack(
            children: [
              // Display image
              Hero(
                tag: widget.file.blobId,
                child: Image(
                  image: provider,
                  semanticLabel: widget.file.name,
                  fit: BoxFit.contain,
                  gaplessPlayback: true,
                ),
              ),
              // Loading indicator
              if (blob.downloading == true)
                const Positioned(
                  right: 8,
                  top: 8,
                  child: SizedBox(
                      width: 20,
                      height: 20,
                      child: CircularProgressIndicator()),
                ),
              // Download failed
              if (blob.failed == true)
                const Positioned(
                  right: 8,
                  top: 8,
                  child: SizedBox(
                      width: 20, height: 20, child: Icon(Icons.sync_problem)),
                ),
            ],
          ),
        ),
      ),
    );
  }
}

enum _ExtraActions {
  openIn,
  delete,
}

class _MediaFileDoc {
  final int index;
  final CardFile file;

  _MediaFileDoc(this.index, this.file);
}

class _PageHeader extends StatelessWidget {
  final int pagesCount;

  const _PageHeader({required this.pagesCount});

  @override
  Widget build(BuildContext context) {
    final pageController = context.watch<PageController>();
    final page =
        (pageController.page?.round() ?? pageController.initialPage) + 1;
    return Text('$page of $pagesCount');
  }
}

class _ArrowButton extends StatefulWidget {
  final Function() onTap;
  final IconData icon;

  const _ArrowButton({super.key, required this.onTap, required this.icon});

  @override
  State<StatefulWidget> createState() => _ArrowButtonState();
}

class _ArrowButtonState extends State<_ArrowButton> {
  var _opacity = 0.0;

  @override
  Widget build(BuildContext context) {
    return InkWell(
      onHover: (hovered) => setState(() => _opacity = hovered ? 1.0 : 0.0),
      onTap: widget.onTap,
      child: AnimatedOpacity(
        duration: const Duration(milliseconds: 200),
        opacity: _opacity,
        child: Padding(
          padding: const EdgeInsets.only(left: 50, right: 50),
          child: Container(
            alignment: Alignment.center,
            decoration: const BoxDecoration(
              shape: BoxShape.circle,
              color: Colors.white,
            ),
            padding: const EdgeInsets.all(16),
            child: Icon(widget.icon),
          ),
        ),
      ),
    );
  }
}
