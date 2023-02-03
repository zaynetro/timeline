import 'package:timeline/bridge_generated.dart';

extension CardViewExt on CardView {
  DateTime get createdAt =>
      DateTime.fromMillisecondsSinceEpoch(createdAtSec * 1000, isUtc: true);

  DateTime get editedAt =>
      DateTime.fromMillisecondsSinceEpoch(editedAtSec * 1000, isUtc: true);

  List<AccLabel> accLabels(AccView acc) {
    final knownLabels = {for (var label in acc.labels) label.id: label};
    return labels.map((l) => knownLabels[l.id]).whereType<AccLabel>().toList()
      ..sort((a, b) => a.name.compareTo(b.name));
  }
}

extension AccViewExt on AccView {
  DateTime get createdAt =>
      DateTime.fromMillisecondsSinceEpoch(createdAtSec * 1000, isUtc: true);
}

extension CardTextAttrsExt on CardTextAttrs {
  Map<String, dynamic> toJson() {
    return {
      'bold': bold,
      'italic': italic,
      'link': link,
      'checked': checked,
      'heading': heading,
      'block': block,
    };
  }
}
