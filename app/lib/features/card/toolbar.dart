import 'dart:io';

import 'package:fleather/fleather.dart';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:timeline/features/card/card_edit.dart';

class FormatTextButton extends StatefulWidget {
  final Function(bool isOpen) onFormatToolbar;

  const FormatTextButton({super.key, required this.onFormatToolbar});

  @override
  State<StatefulWidget> createState() => _FormatTextButtonState();
}

class _FormatTextButtonState extends State<FormatTextButton> {
  OverlayEntry? overlayEntry;

  void hideToolbar() {
    overlayEntry?.remove();
    overlayEntry = null;
    widget.onFormatToolbar(false);
  }

  @override
  void dispose() {
    hideToolbar();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return IconButton(
      onPressed: () {
        final editorTheme = FleatherThemeData.fallback(context);
        final cardEdit = context.read<TimelineCardEdit>();

        hideToolbar();
        widget.onFormatToolbar(true);

        overlayEntry = OverlayEntry(
          builder: (BuildContext context) {
            return Align(
              alignment: Alignment.bottomLeft,
              child: _FormatToolbar(
                editorTheme: editorTheme,
                cardEdit: cardEdit,
                onClose: hideToolbar,
              ),
            );
          },
        );

        Overlay.of(context).insert(overlayEntry!);
      },
      tooltip: 'Format text',
      icon: const Icon(Icons.text_format),
    );
  }
}

class _FormatToolbar extends StatelessWidget {
  final FleatherThemeData editorTheme;
  final TimelineCardEdit cardEdit;
  final Function() onClose;

  const _FormatToolbar(
      {super.key,
      required this.editorTheme,
      required this.cardEdit,
      required this.onClose});

  Widget _buildHeadingButton(
    BuildContext context, {
    required Function() onPressed,
    required String text,
    required TextStyle style,
    required bool isSelected,
  }) {
    final theme = Theme.of(context);
    final color = theme.colorScheme.primary.withOpacity(0.12);
    const minSize = Size.square(48);
    const padding = EdgeInsets.symmetric(horizontal: 8);
    const borderRadius = BorderRadius.all(Radius.circular(8));

    return TextButton(
      onPressed: onPressed,
      style: ButtonStyle(
        backgroundColor: isSelected ? MaterialStatePropertyAll(color) : null,
        minimumSize: const MaterialStatePropertyAll(minSize),
        padding: const MaterialStatePropertyAll(padding),
        shape: const MaterialStatePropertyAll(RoundedRectangleBorder(
          borderRadius: borderRadius,
        )),
      ),
      child: Text(text, style: style),
    );
  }

  @override
  Widget build(BuildContext context) {
    final screenSize = MediaQuery.of(context).size;
    final dense = screenSize.width < 700;
    final mediaPadding = MediaQuery.of(context).padding;
    // mediaPadding.bottom is not-zero in case there is "unsafe" area (e.g a notch on iPhones)
    // Basically, this is a partial implementation of SafeArea.
    final bottomPadding =
        mediaPadding.bottom > 0 ? mediaPadding.bottom : (dense ? 8.0 : 16.0);

    return Container(
      padding: EdgeInsets.only(
        top: dense ? 4 : 8,
        // We want to see the toolbar when virtual keyboard is opened
        bottom: MediaQuery.of(context).viewInsets.bottom,
      ),
      decoration: const BoxDecoration(
        color: Colors.white,
        boxShadow: [
          BoxShadow(
            color: Colors.black38,
            blurRadius: 3.0,
            offset: Offset(0.0, 0.75),
          )
        ],
      ),
      child: AnimatedBuilder(
        animation: cardEdit.controller,
        builder: (context, child) {
          final selectionStyle = cardEdit.controller.getSelectionStyle();

          return Column(
            mainAxisSize: MainAxisSize.min,
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              SingleChildScrollView(
                scrollDirection: Axis.horizontal,
                child: Row(
                  children: [
                    const SizedBox(width: 8),
                    _buildHeadingButton(
                      context,
                      onPressed: () {
                        cardEdit.toggleFormat(ParchmentAttribute.h1);
                      },
                      text: 'Title',
                      style: editorTheme.heading1.style,
                      isSelected:
                          selectionStyle.containsSame(ParchmentAttribute.h1),
                    ),
                    _buildHeadingButton(
                      context,
                      onPressed: () {
                        cardEdit.toggleFormat(ParchmentAttribute.h2);
                      },
                      text: 'Heading',
                      style: editorTheme.heading2.style,
                      isSelected:
                          selectionStyle.containsSame(ParchmentAttribute.h2),
                    ),
                    _buildHeadingButton(
                      context,
                      onPressed: () {
                        cardEdit.toggleFormat(ParchmentAttribute.h3);
                      },
                      text: 'Subheading',
                      style: editorTheme.heading3.style,
                      isSelected:
                          selectionStyle.containsSame(ParchmentAttribute.h3),
                    ),
                    _buildHeadingButton(
                      context,
                      onPressed: () {
                        cardEdit.resetHeadingFormat();
                      },
                      text: 'Body',
                      style: editorTheme.paragraph.style,
                      isSelected:
                          !selectionStyle.contains(ParchmentAttribute.heading),
                    ),
                    const SizedBox(width: 8),
                  ],
                ),
              ),
              SizedBox(height: dense ? 4 : 12),
              Padding(
                padding: const EdgeInsets.symmetric(horizontal: 16),
                child: Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    ToggleButtons(
                      onPressed: (index) {
                        switch (index) {
                          case 0:
                            return cardEdit
                                .toggleFormat(ParchmentAttribute.bold);
                          case 1:
                            return cardEdit
                                .toggleFormat(ParchmentAttribute.italic);
                          case 2:
                            return cardEdit
                                .toggleFormat(ParchmentAttribute.underline);
                          case 3:
                            return cardEdit
                                .toggleFormat(ParchmentAttribute.strikethrough);
                        }
                      },
                      constraints:
                          const BoxConstraints(minWidth: 40, minHeight: 40),
                      borderRadius: const BorderRadius.all(Radius.circular(8)),
                      isSelected: [
                        selectionStyle.containsSame(ParchmentAttribute.bold),
                        selectionStyle.containsSame(ParchmentAttribute.italic),
                        selectionStyle
                            .containsSame(ParchmentAttribute.underline),
                        selectionStyle
                            .containsSame(ParchmentAttribute.strikethrough),
                      ],
                      children: const [
                        Icon(Icons.format_bold),
                        Icon(Icons.format_italic),
                        Icon(Icons.format_underline),
                        Icon(Icons.format_strikethrough),
                      ],
                    ),
                    TextButton(
                      onPressed: onClose,
                      child: const Text('Close'),
                    ),
                  ],
                ),
              ),
              SizedBox(height: bottomPadding),
            ],
          );
        },
      ),
    );
  }
}
