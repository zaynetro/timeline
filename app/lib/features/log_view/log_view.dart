import 'dart:async';
import 'dart:convert';
import 'dart:io';

import 'package:flutter/material.dart';
import 'package:open_filex/open_filex.dart';
import 'package:timeline/common/models/logger.dart';
import 'package:timeline/common/models/phase_info.dart';
import 'package:url_launcher/url_launcher.dart';

class LogViewPage extends StatefulWidget {
  const LogViewPage({super.key});

  @override
  State<StatefulWidget> createState() => _LogViewPageState();
}

class _LogViewPageState extends State<LogViewPage> {
  final scroll = ScrollController();
  var _filter = '';
  Timer? _setFilterTimer;
  var _logLines = <String>[];

  @override
  void initState() {
    super.initState();
    _readLogFile();
  }

  void _readLogFile() {
    final logFile = logger.fileOutput?.file;
    if (logFile != null) {
      Stream<List<int>> inputStream = logFile.openRead();

      var lines = <String>[];
      inputStream
          .transform(utf8.decoder)
          .transform(const LineSplitter())
          .listen((String line) {
        lines.add(line);
      }, onDone: () {
        setState(() => _logLines = lines);
      }, onError: (e) {
        logger.warn(e.toString());
      });
    }
  }

  @override
  void dispose() {
    _setFilterTimer?.cancel();
    scroll.dispose();
    super.dispose();
  }

  _setFilter(String value) {
    // Debounce these calls
    if (_setFilterTimer?.isActive ?? false) {
      _setFilterTimer?.cancel();
    }

    _setFilterTimer = Timer(const Duration(seconds: 2), () {
      setState(() => _filter = value.toLowerCase());
      _setFilterTimer?.cancel();
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Logs'),
        actions: [
          TextButton.icon(
            onPressed: () async {
              await logger.truncateFile();
              if (!mounted) return;

              // Rebuild widget
              _readLogFile();
            },
            icon: const Icon(Icons.clear_all),
            label: const Text('Truncate file'),
          ),
          TextButton.icon(
            onPressed: () {
              final logFilePath = logger.fileOutput?.filePath;
              if (logFilePath != null) {
                if (Platform.isAndroid || Platform.isIOS) {
                  OpenFilex.open(logFilePath);
                } else {
                  launchUrl(Uri.parse('file:$logFilePath'));
                }
              }
            },
            icon: const Icon(Icons.open_in_new),
            label: const Text('Open in'),
          ),
        ],
      ),
      body: _LogView(scroll: scroll, filter: _filter, logLines: _logLines),
      floatingActionButton: FloatingActionButton(
        onPressed: () {
          // We want to scroll to the bottom when controller is attached to a view.
          if (scroll.positions.isNotEmpty) {
            scroll.jumpTo(0);
          }
        },
        child: const Icon(Icons.arrow_downward),
      ),
      bottomNavigationBar: Padding(
        padding: EdgeInsets.only(
          top: 8,
          left: 16,
          right: 16,
          // We want to see TextField when virtual keyboard is opened
          bottom: MediaQuery.of(context).viewInsets.bottom + 8,
        ),
        child: TextField(
          onChanged: _setFilter,
          decoration: const InputDecoration(
            hintText: 'Filter logs',
          ),
        ),
      ),
    );
  }
}

class _LogView extends StatelessWidget {
  final ScrollController scroll;
  final String filter;
  final List<String> logLines;

  const _LogView(
      {required this.scroll, required this.filter, required this.logLines});

  @override
  Widget build(BuildContext context) {
    final filteredLines = logLines.reversed
        .where((line) =>
            filter.isEmpty ? true : line.toLowerCase().contains(filter))
        .toList();

    return Padding(
      padding: const EdgeInsets.all(8),
      child: filteredLines.isEmpty
          ? const Center(child: Text('No logs found'))
          : SelectionArea(
              child: Scrollbar(
                controller: scroll,
                child: ListView.builder(
                  reverse: true,
                  controller: scroll,
                  itemCount: filteredLines.length,
                  itemBuilder: (context, i) =>
                      _LogLineView(line: _LogLine.parse(filteredLines[i])),
                ),
              ),
            ),
    );
  }
}

class _LogLineView extends StatelessWidget {
  final _LogLine line;

  const _LogLineView({required this.line});

  @override
  Widget build(BuildContext context) {
    Color? levelColor;
    if (line.level == LogLevel.error) {
      levelColor = Colors.red;
    } else if (line.level == LogLevel.warn) {
      levelColor = Colors.orange;
    } else if (line.level == LogLevel.info) {
      levelColor = Colors.blueGrey[700];
    } else if (line.level == LogLevel.trace) {
      levelColor = Colors.grey[400];
    } else {
      levelColor = Colors.grey[600];
    }

    return Row(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        Text(
          '${line.date} ',
          style: const TextStyle(fontSize: 9, fontFamily: 'monospace'),
        ),
        if (line.level != null)
          Text(
            ' ${line.level!.name.toUpperCase().padLeft(5)} ',
            style: TextStyle(
              fontSize: 9,
              fontFamily: 'monospace',
              color: levelColor,
            ),
          ),
        Flexible(
          child: Text(
            line.text,
            style: const TextStyle(fontSize: 10, fontFamily: 'monospace'),
          ),
        ),
      ],
    );
  }
}

class _LogLine {
  final String date;
  final LogLevel? level;
  final String text;

  _LogLine({required this.date, required this.level, required this.text});

  static _LogLine parse(String line) {
    final level = Logger.levelFromLine(line);
    if (level == null) {
      // Log entry spans across multiple lines (ignore parsing)
      return _LogLine(date: '', level: null, text: line);
    }

    try {
      final dateEnd = line.indexOf(' ');
      final date = line.substring(0, dateEnd);

      // LogLevel is a padded string with max len of 5 and spaces around.
      final levelStart = dateEnd;
      final levelEnd = levelStart + 7;
      final text = line.substring(levelEnd);
      return _LogLine(date: date, level: level, text: text);
    } catch (e) {
      return _LogLine(date: '', level: LogLevel.error, text: line);
    }
  }
}
