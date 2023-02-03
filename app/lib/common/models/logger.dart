import 'dart:io';

import 'package:flutter/foundation.dart';
import 'package:path/path.dart' as p;

enum LogLevel {
  trace(0),
  debug(1),
  info(2),
  warn(3),
  error(4);

  final int order;
  const LogLevel(this.order);
}

const _maxFileSize = 1024 * 1024 * 5 /* MBs */;

class Logger {
  final LogLevel _level;
  final List<LogOutput> _listeners = [];

  Logger(this._level, {bool stdOutput = false}) {
    if (stdOutput) {
      _listeners.add(_StdOutput());
    }
  }

  void addFileOutput(String directory) {
    final logFilePath = p.join(directory, "app.log");
    info('Using $logFilePath for logs');
    final file = File(logFilePath);

    var mode = FileMode.writeOnlyAppend;
    var logSize = 0;
    if (file.existsSync()) {
      logSize = file.lengthSync();
      if (logSize > _maxFileSize) {
        mode = FileMode.writeOnly;
      }
    }

    _listeners.add(FileLogOutput(logFilePath, file, mode));
    if (mode == FileMode.writeOnly) {
      info("Truncated log file automatically (file_size(before)=$logSize)");
    }
  }

  FileLogOutput? get fileOutput {
    for (var listener in _listeners) {
      if (listener is FileLogOutput) {
        return listener;
      }
    }
    return null;
  }

  Future<void> truncateFile() async {
    await fileOutput?._truncate();
    debug('Truncated log file');
  }

  void debug(String line) => _flutterEvent(LogLevel.debug, line);
  void info(String line) => _flutterEvent(LogLevel.info, line);
  void warn(String line) => _flutterEvent(LogLevel.warn, line);
  void error(String line) => _flutterEvent(LogLevel.error, line);

  // An log event coming from Rust SDK
  void nativeEvent(String line) {
    final level = levelFromLine(line) ?? LogLevel.trace;
    _event(LogEvent(level, line, hasNewLine: true));
  }

  void _flutterEvent(LogLevel level, String line) {
    final now = DateTime.now().toUtc().toIso8601String();
    final paddedLevel = level.name.toUpperCase().padLeft(5);
    final text = '$now $paddedLevel flutter: $line';
    _event(LogEvent(level, text));
  }

  void _event(LogEvent event) {
    if (event.level.order < _level.order) {
      return;
    }

    for (var listener in _listeners) {
      try {
        listener.event(event);
      } catch (e) {
        debugPrint('Failed to push log event: $e');
      }
    }
  }

  static LogLevel? levelFromLine(String line) {
    if (line.contains(" ERROR ")) {
      return LogLevel.error;
    } else if (line.contains(" WARN ")) {
      return LogLevel.warn;
    } else if (line.contains(" INFO ")) {
      return LogLevel.info;
    } else if (line.contains(" DEBUG ")) {
      return LogLevel.debug;
    } else if (line.contains(" TRACE ")) {
      return LogLevel.trace;
    } else {
      return null;
    }
  }
}

class LogEvent {
  final LogLevel level;
  final String line;
  final bool hasNewLine;

  LogEvent(this.level, this.line, {this.hasNewLine = false});
}

abstract class LogOutput {
  void event(LogEvent event);
  void destroy() {}
}

class _StdOutput extends LogOutput {
  @override
  void event(LogEvent event) {
    if (Platform.isAndroid) {
      debugPrint(event.line);
    } else {
      stdout.write(event.line);
      if (!event.hasNewLine) {
        stdout.writeln();
      }
    }
  }
}

class FileLogOutput extends LogOutput {
  final String filePath;
  final File file;
  IOSink? _sink;

  FileLogOutput(this.filePath, this.file, FileMode mode)
      : _sink = file.openWrite(mode: mode);

  @override
  void event(LogEvent event) {
    _sink?.write(event.line);
    if (!event.hasNewLine) {
      _sink?.writeln();
    }
  }

  // Clear current log file and start appending
  Future<void> _truncate() async {
    final oldSink = _sink;
    _sink = null;

    await oldSink?.close();
    _sink = file.openWrite(mode: FileMode.writeOnly);
  }

  @override
  void destroy() async {
    await _sink?.flush();
    await _sink?.close();
  }
}
