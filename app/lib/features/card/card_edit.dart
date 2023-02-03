import 'dart:async';
import 'dart:math';

import 'package:fleather/fleather.dart';
import 'package:flutter/widgets.dart';
import 'package:quill_delta/quill_delta.dart';
import 'package:timeline/bridge_generated.dart';
import 'package:timeline/common/models/dispatcher.dart';
import 'package:timeline/common/models/phase_info.dart';
import 'package:timeline/features/card/image_preview.dart';

class TimelineCardEdit extends ChangeNotifier {
  CardView card;
  final AppEventDispatcher dispatcher;
  final Native native;

  late final FleatherController controller;
  late Delta lastSaved;
  bool _docChanged = false;
  bool? cardPreview;

  TimelineCardEdit(this.card, this.dispatcher, this.native,
      {this.cardPreview}) {
    dispatcher.addListener(_onAppEvent);

    final delta = Delta();

    for (var block in card.blocks) {
      final contentView = block.view;
      if (contentView is ContentView_Text) {
        final text = contentView.field0.value;
        final cardAttrs = contentView.field0.attrs;

        if (cardAttrs != null) {
          final attrs = <String, dynamic>{};
          if (cardAttrs.bold == true) {
            attrs[ParchmentAttribute.bold.key] = true;
          }
          if (cardAttrs.italic == true) {
            attrs[ParchmentAttribute.italic.key] = true;
          }
          if (cardAttrs.underline == true) {
            attrs[ParchmentAttribute.underline.key] = true;
          }
          if (cardAttrs.strikethrough == true) {
            attrs[ParchmentAttribute.strikethrough.key] = true;
          }
          if (cardAttrs.link != null) {
            attrs[ParchmentAttribute.link.key] = cardAttrs.link;
          }
          if (cardAttrs.checked == true) {
            attrs[ParchmentAttribute.checked.key] = true;
          }
          if (cardAttrs.block != null) {
            attrs[ParchmentAttribute.block.key] = cardAttrs.block;
          }
          if (cardAttrs.heading != null) {
            attrs[ParchmentAttribute.heading.key] = cardAttrs.heading;
          }

          if (attrs.isNotEmpty) {
            delta.insert(text, attrs);
          } else {
            delta.insert(text);
          }
        } else {
          delta.insert(text);
        }
      } else if (contentView is ContentView_File) {
        delta.insert(SpanEmbed('bolik-file', data: {'obj': contentView.field0})
            .toJson());
      }
    }

    // Editor forces a few restrictions on Delta. Here we enforce them on a new delta.
    // Old delta will be used so that we can calculate correct diff when storing the doc.
    final editorDelta = Delta.from(delta);
    if (editorDelta.isEmpty) {
      _docChanged = true;
      editorDelta.insert('\n');
    } else {
      // Delta must end with a newline
      if (editorDelta.last.data is String &&
          (editorDelta.last.data as String).endsWith('\n')) {
        // All good
      } else {
        _docChanged = true;
        editorDelta.insert('\n');
      }
    }

    final heuristics = ParchmentHeuristics(
      formatRules: [],
      insertRules: [
        // Fleather uses String::len instead of Characters::len which poses a problem to us
        // when user inserts emojis. Emojis' len is 2 in the editor but 1 in our SDK.
        // Because of this inconsistency all delta indices after emoji are skewed and
        // modify wrong data on the SDK side.
        // As a workaround I am using a single insert rule that first replaces emojis
        // and then passes updated data to other insert rules.
        _ReplaceEmojiCatchAllRule([
          _ForceNewlineForInsertsAroundBolikFileRule(),
          ...ParchmentHeuristics.fallback.insertRules
        ])
      ],
      deleteRules: [_EnsureBolikFileRule()],
    ).merge(ParchmentHeuristics(
      formatRules: ParchmentHeuristics.fallback.formatRules,
      insertRules: [],
      deleteRules: ParchmentHeuristics.fallback.deleteRules,
    ));
    final doc =
        ParchmentDocument.fromDelta(editorDelta, heuristics: heuristics);
    lastSaved = delta;

    doc.changes.listen((event) {
      if (event.source == ChangeSource.local) {
        // Queue save
        _queueChange(event);
      }
    });

    controller = FleatherController(doc);

    if (_docChanged) {
      _save();
    }
  }

  ParchmentDocument get doc => controller.document;

  Future<void> close() async {
    await _cleanup();

    try {
      await native.closeCard(cardId: card.id);
      controller.dispose();
    } catch (e) {
      logger.warn("Failed to close card: $e");
    }
  }

  bool readonly(AccView account) {
    if (cardPreview == true) {
      return true;
    }

    // Was card deleted
    if (inBin()) {
      return true;
    }

    // Do we have only Read rights
    for (var entry in card.acl.accounts) {
      if (entry.accountId == account.id) {
        return entry.rights == AclRights.Read;
      }
    }

    return false;
  }

  bool inBin() {
    return card.labels.any((l) => l.id == deletedLabelId);
  }

  bool get isEmpty {
    if (card.blocks.isEmpty) {
      return true;
    }

    if (card.blocks.length > 1) {
      return false;
    }

    final view = card.blocks[0].view;
    if (view is ContentView_Text) {
      return view.field0.value.trim().isEmpty;
    }

    return false;
  }

  Future<void> _cleanup() async {
    dispatcher.removeListener(_onAppEvent);
    _save();
  }

  _onAppEvent(OutputEvent event) async {}

  void _queueChange(ParchmentChange change) {
    if (_docChanged) {
      // We already queued the save
      return;
    }

    _docChanged = true;
    queueSave(delay: const Duration(seconds: 5));
  }

  /// Add a minimal delay by default. We need this delay because doc change might
  /// not have been propagated through events yet.
  Future<void> queueSave(
      {Duration delay = const Duration(milliseconds: 100)}) async {
    Future.delayed(delay, () async {
      _save();
      // notifyListeners();
    });
  }

  Future<void> _save() async {
    if (!_docChanged) {
      return;
    }

    final pendingSave = doc.toDelta();
    final diff = lastSaved.diff(pendingSave);

    var index = 0;
    final cardChanges = <CardChange>[];

    insertView(ContentView view) {
      cardChanges.add(CardChange.insert(CardBlock(
        position: index,
        view: view,
      )));
    }

    insertText(String text, {CardTextAttrs? attrs}) {
      insertView(ContentView.text(CardText(value: text, attrs: attrs)));
    }

    format(int len, CardTextAttrs attrs) {
      cardChanges
          .add(CardChange.format(position: index, len: len, attributes: attrs));
    }

    remove(int len) {
      cardChanges.add(CardChange.remove(position: index, len: len));
    }

    for (var op in diff.toList()) {
      if (op.isRetain) {
        // Retain (skip)
        final skip = op.length;

        // Retain operation might contain attributes.
        final cardAttrs = buildCardTextAttrs(op.attributes);
        if (cardAttrs != null) {
          format(skip, cardAttrs);
        }

        index += skip;
      } else if (op.isInsert) {
        // Insert
        final value = op.value;
        if (value is String) {
          // Text
          final cardAttrs = buildCardTextAttrs(op.attributes);
          insertText(value, attrs: cardAttrs);
          index += value.length;
        } else if (value is Map) {
          // Embed
          final type = value[EmbeddableObject.kTypeKey];
          if (type == 'bolik-file') {
            final cardFile = value['obj'] as CardFile;
            insertView(ContentView.file(cardFile));
          } else {
            // Unsupported embed
            insertText('?');
          }

          index += 1;
        } else {
          // Unknown embed
          insertText('?');
          index += 1;
        }
      } else if (op.isDelete) {
        // Delete
        final len = op.length;
        remove(len);
        // Here it is important not to increment the index.
      }
    }

    if (cardChanges.isNotEmpty) {
      card = await native.editCard(cardId: card.id, changes: cardChanges);
      // notifyListeners();
    }

    _docChanged = false;
    lastSaved = pendingSave;
  }

  /// Attach file to the end of the document.
  /// There a several possible combinations.
  /// 1. Document ends with attributed whitespace (e.g checklist)
  /// 2. Document ends with text
  /// 3. Document ends with embed (there will be some trailing whitespace though)
  /// In case of (1) and (2) we want to insert newlines before the file.
  /// In case of (3) we want to insert the file on the same line after the embed
  /// of the same type.
  Future<void> attachFile(String path) async {
    final cardFile = await native.saveFile(cardId: card.id, path: path);
    final delta = doc.toDelta();
    String? lastIsFile;
    var insertAt = doc.length;
    final isImage = isImageFile(cardFile.name);

    for (var i = delta.length - 1; i >= 0; i -= 1) {
      final op = delta[i];
      if (op.data is String) {
        final data = op.data as String;

        // Skip empty inserts (only whitespace and no attributes)
        if (data.trim().isEmpty && op.attributes == null) {
          insertAt -= data.length;
          continue;
        }

        // File embed is not last
        break;
      } else if (op.data is Map) {
        try {
          final data = op.data as Map;
          if (data[EmbeddableObject.kTypeKey] == 'bolik-file') {
            final file = data['obj'] as CardFile;
            if (isImageFile(file.name)) {
              lastIsFile = 'image';
            } else {
              lastIsFile = 'file';
            }
          }
        } catch (e) {
          logger.warn('Failed to process second to last char: $e');
        }
        break;
      }
    }

    if (lastIsFile == 'image' && isImage) {
      // Insert same line
    } else if (lastIsFile == 'file' && !isImage) {
      // Insert same line
    } else {
      // Insert newlines first
      final lastAttrs = delta[delta.length - 1].attributes;
      if (lastAttrs?.containsKey(ParchmentAttribute.block.key) ?? false) {
        // Insert extra new line to reset block attributes (e.g after checklist)
        _appendText('\n');
      }
      _appendText('\n');
      insertAt = doc.length - 1;
    }

    doc.insert(
        insertAt,
        SpanEmbed('bolik-file', data: {
          'obj': cardFile,
        }));

    final last = delta[delta.length - 1];
    if (last.data is String) {
      final lastData = last.data as String;
      if (lastData.endsWith('\n\n')) {
        // We have enough newlines at the end of the doc
      } else {
        _appendText('\n');
      }
    } else {
      _appendText('\n');
    }

    controller.updateSelection(TextSelection.collapsed(offset: doc.length));
  }

  void removeFile(CardFile file) {
    final delta = doc.toDelta();
    var index = 0;
    for (var op in delta.toList()) {
      if (op.data is Map) {
        final data = op.data as Map;
        if (data['obj'] == file) {
          doc.delete(index, 1);
          controller.updateSelection(TextSelection.collapsed(offset: index));
          queueSave();
          return;
        }
      }

      index += op.length;
    }
  }

  /// Append text to the doc
  void _appendText(String text) {
    doc.insert(max(0, doc.length - 1), text);
  }

  Future<void> addTask() async {
    _appendText('\n');
    controller.updateSelection(TextSelection.collapsed(offset: doc.length));
    controller.formatText(
        doc.length - 1, 1, ParchmentAttribute.block.checkList);
  }

  Future<void> moveToBin() async {
    await _cleanup();
    await native.moveCardToBin(cardId: card.id);
  }

  Future<void> moveToBinAll() async {
    await _cleanup();
    await native.moveCardToBinAll(cardId: card.id);
  }

  /// Create a copy of deleted card.
  Future<CardView> restore() async {
    final newCard = await native.restoreFromBin(cardId: card.id);
    return newCard;
  }

  Future<void> editCollaborators(List<CollaboratorChange> changes) async {
    if (changes.isNotEmpty) {
      card = await native.editCollaborators(
        cardId: card.id,
        changes: changes,
      );
      notifyListeners();
    }
  }

  Future<void> addLabel(String labelId) async {
    card = await native.addCardLabel(cardId: card.id, labelId: labelId);
    notifyListeners();
  }

  Future<void> removeLabel(String labelId) async {
    card = await native.removeCardLabel(cardId: card.id, labelId: labelId);
    notifyListeners();
  }

  void toggleFormat(ParchmentAttribute attr) {
    final selectionStyle = controller.getSelectionStyle();

    if (selectionStyle.containsSame(attr)) {
      controller.formatSelection(attr.unset);
    } else {
      // Block and heading are exclusive
      if (selectionStyle.contains(ParchmentAttribute.block) &&
          attr.key == ParchmentAttribute.heading.key) {
        controller.formatSelection(ParchmentAttribute.block.unset);
      } else if (selectionStyle.contains(ParchmentAttribute.heading) &&
          attr.key == ParchmentAttribute.block.key) {
        controller.formatSelection(ParchmentAttribute.heading.unset);
      }
      controller.formatSelection(attr);
    }
  }

  void resetHeadingFormat() {
    controller.formatSelection(ParchmentAttribute.heading.unset);
  }
}

class CardBlobs extends ChangeNotifier {
  final String cardId;
  final AppEventDispatcher dispatcher;
  final Native native;

  // Blob ID to loading blob
  final Map<String, MaybeBlobPath> blobs = {};

  CardBlobs(this.cardId, this.dispatcher, this.native) {
    dispatcher.addListener(_onAppEvent);
  }

  @override
  void dispose() {
    dispatcher.removeListener(_onAppEvent);
    super.dispose();
  }

  _onAppEvent(OutputEvent event) async {
    if (event is OutputEvent_DownloadCompleted) {
      blobs[event.blobId] = MaybeBlobPath(path: event.path);
      notifyListeners();
    } else if (event is OutputEvent_DownloadFailed) {
      blobs[event.blobId] = MaybeBlobPath(failed: true);
      notifyListeners();
    }
  }

  void downloadFile(CardFile file) async {
    if (blobs.containsKey(file.blobId)) {
      return;
    }

    blobs[file.blobId] = MaybeBlobPath();

    try {
      final result = await native.downloadFile(
          cardId: cardId, blobId: file.blobId, deviceId: file.deviceId);

      blobs[file.blobId] = MaybeBlobPath(
        path: result.path,
        downloading: result.downloadStarted,
      );
      notifyListeners();
    } catch (e) {
      logger.warn("Failed to download a file ${file.blobId}: $e");
      blobs[file.blobId] = MaybeBlobPath(failed: true);
      notifyListeners();
    }
  }
}

class MaybeBlobPath {
  String? path;
  final bool downloading;
  final bool failed;

  MaybeBlobPath({this.path, this.downloading = false, this.failed = false});
}

class CardDragState extends ChangeNotifier {
  String? draggingId;

  void move(TimelineCardEdit cardEdit, {String? beforeId, String? afterId}) {}

  void start(TimelineCardEdit cardEdit, String contentId) {
    cardEdit._save();
    draggingId = contentId;
    notifyListeners();
  }

  bool get ongoing => draggingId != null;

  Future<void> endDrag(TimelineCardEdit cardEdit) async {
    if (draggingId != null) {
      draggingId = null;
      notifyListeners();
    }
  }
}

// Content that is being dragged.
class ContentAvatar {
  final String contentId;

  ContentAvatar(this.contentId);
}

CardTextAttrs? buildCardTextAttrs(Map<String, dynamic>? attrs) {
  if (attrs == null) {
    return null;
  }

  bool? readBool(String key) {
    if (attrs.containsKey(key)) {
      final value = attrs[key] as bool?;
      return value ?? false;
    }
    return null;
  }

  int? readInt(String key) {
    if (attrs.containsKey(key)) {
      final value = attrs[key] as int?;
      return value ?? 0;
    }
    return null;
  }

  String? readStr(String key) {
    if (attrs.containsKey(key)) {
      final value = attrs[key] as String?;
      return value ?? '';
    }
    return null;
  }

  // Example values:
  // {b: true}  <-- Bold text
  // {b: null}  <-- Undo bold
  // {block: ul}    <-- List item
  // {block: null}  <-- Undo list item

  return CardTextAttrs(
    bold: readBool(ParchmentAttribute.bold.key),
    italic: readBool(ParchmentAttribute.italic.key),
    underline: readBool(ParchmentAttribute.underline.key),
    strikethrough: readBool(ParchmentAttribute.strikethrough.key),
    block: readStr(ParchmentAttribute.block.key),
    checked: readBool(ParchmentAttribute.checked.key),
    link: readStr(ParchmentAttribute.link.key),
    heading: readInt(ParchmentAttribute.heading.key),
  );
}

class _ReplaceEmojiCatchAllRule extends InsertRule {
  final List<InsertRule> rules;

  _ReplaceEmojiCatchAllRule(this.rules);

  @override
  Delta? apply(Delta document, int index, Object data) {
    Object fixedData = data;
    if (data is String) {
      final chars = Characters(data);
      if (data.length != chars.length) {
        String fixedStr = '';
        for (var char in chars) {
          if (char.length > 1) {
            const replacement = '?';
            logger.debug("Replacing emoji=$char with=$replacement !");
            fixedStr += replacement;
          } else {
            fixedStr += char;
          }
        }
        fixedData = fixedStr;
      }
    }

    // Fallback
    for (var rule in rules) {
      final delta = rule.apply(document, index, fixedData);
      if (delta != null) {
        return delta;
      }
    }
    return null;
  }
}

/// Insert a new line before and after bolik-file embed.
class _ForceNewlineForInsertsAroundBolikFileRule extends InsertRule {
  @override
  Delta? apply(Delta document, int index, Object data) {
    if (data is! String) return null;

    final iter = DeltaIterator(document);
    final previous = iter.skip(index);
    final target = iter.next();
    final cursorBeforeFile = _isBolikFile(target.data);
    final cursorAfterFile = previous != null && _isBolikFile(previous.data);

    if (cursorBeforeFile || cursorAfterFile) {
      final delta = Delta()..retain(index);
      if (cursorAfterFile && !data.startsWith('\n')) {
        delta.insert('\n');
      }
      delta.insert(data);
      if (cursorBeforeFile && !data.endsWith('\n')) {
        delta.insert('\n');
        // TODO: in this case we need to move the cursor back..
      }
      return delta;
    }
    return null;
  }

  bool _isBolikFile(Object data) {
    if (data is EmbeddableObject) {
      return data.type == 'bolik-file';
    }
    if (data is Map) {
      return data[EmbeddableObject.kTypeKey] == 'bolik-file';
    }
    return false;
  }
}

/// Prevent merging line with bolik-file embeds with other lines.
class _EnsureBolikFileRule extends DeleteRule {
  @override
  Delta? apply(Delta document, int index, int length) {
    final iter = DeltaIterator(document);

    final prev = iter.skip(index);
    // Text that we want to delete
    final target = iter.skip(length);

    final targetText = target?.data is String ? target!.data as String : '';
    if (targetText.endsWith('\n') || targetText.startsWith('\n')) {
      // Text or block that comes after deleted text
      final next = iter.next();

      final prevText = prev?.data is String ? prev!.data as String : '';
      final nextText = next.data is String ? next.data as String : '';
      if (prev == null || prevText.endsWith('\n') || nextText == '\n') {
        // Allow deleting in-between empty lines
        return null;
      }

      final canPrevGroup = _isBolikFile(prev.data);
      final canNextGroup = _isBolikFile(next.data);
      if (canPrevGroup && canNextGroup) {
        // Allow joining embeds that support grouping
        return null;
      }

      final isPrevBlock = _isBolikFile(prev.data);
      final isNextBlock = _isBolikFile(next.data);

      if (isPrevBlock) {
        if (nextText.startsWith('\n')) {
          // Block + \n + text  --> all good
          return _withDeletion(index, length);
        }

        // Block + text  --> keep single newline from target
        return _withSingleNewline(index, length, targetText);
      }

      if (isNextBlock) {
        if (prevText.endsWith('\n')) {
          // Text + \n + block  --> all good
          return _withDeletion(index, length);
        }

        // Text + block  --> keep single newline from target
        return _withSingleNewline(index, length, targetText);
      }
    }

    return null;
  }

  bool _isBolikFile(Object data) {
    if (data is EmbeddableObject) {
      return data.type == 'bolik-file';
    }
    if (data is Map) {
      return data[EmbeddableObject.kTypeKey] == 'bolik-file';
    }
    return false;
  }

  static Delta _withDeletion(int index, int length) {
    return Delta()
      ..retain(index)
      ..delete(length);
  }

  /// Delete target text but a single newline character.
  static Delta _withSingleNewline(int index, int length, String targetText) {
    if (targetText == '\n') {
      // No changes needed
      return Delta();
    }

    // TODO: we need to clear attributes from kept newline
    //       e.g. try deleting list next to embed

    if (targetText.startsWith('\n')) {
      // Keep leading newline
      return Delta()
        ..retain(index + 1)
        ..delete(length - 2);
    } else {
      // Keep trailing newline
      return Delta()
        ..retain(index)
        ..delete(length - 1);
    }
  }
}

// void _prettyPrintDelta(Delta delta) {
//   print('!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!');
//   print('Doc:');
//   var debugText = '\n';
//   var count = 0;
//   for (var i = 0; i < delta.length; i += 1) {
//     final op = delta[i];
//     if (op.value is String) {
//       final attrs = op.attributes;
//       var shortAttrs = '';
//       final block = attrs?[ParchmentAttribute.block.key] as String?;
//       final checked = attrs?[ParchmentAttribute.checked.key] as bool?;
//       if (block != null) {
//         shortAttrs += ' block=$block';
//       }
//       if (checked != null) {
//         shortAttrs += ' checked=$checked';
//       }

//       final s = op.value as String;
//       for (var c in s.characters) {
//         if (c == '\n') {
//           debugText += '$count: ⮰$shortAttrs\n';
//         } else {
//           debugText += '$count: $c ';
//         }
//         count += 1;
//       }
//     } else {
//       debugText += '$count: ?';
//       count += 1;
//     }
//   }
//   print(debugText);
//   print('!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!');
// }
