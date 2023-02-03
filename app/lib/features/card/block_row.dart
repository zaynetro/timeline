import 'package:flutter/material.dart';

class CardBlockRow extends StatelessWidget {
  final Widget child;
  final double maxContentWidth;

  const CardBlockRow(
      {super.key, required this.child, required this.maxContentWidth});

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 8, horizontal: 16),
      child: ConstrainedBox(
        constraints: BoxConstraints(maxWidth: maxContentWidth),
        child: Align(alignment: Alignment.topLeft, child: child),
      ),
    );
  }
}
