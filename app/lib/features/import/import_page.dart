import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:timeline/bridge_generated.dart';
import 'package:timeline/common/models/app_state.dart';
import 'package:timeline/common/models/phase_info.dart';
import 'package:url_launcher/url_launcher.dart';

class ImportPage extends StatelessWidget {
  const ImportPage({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Import data'),
      ),
      body: Center(
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 16),
          constraints: const BoxConstraints(maxWidth: 500),
          child: const _ImportContent(),
        ),
      ),
    );
  }
}

class _ImportContent extends StatefulWidget {
  const _ImportContent({super.key});

  @override
  State<StatefulWidget> createState() => _ImportContentState();
}

class _ImportContentState extends State<_ImportContent> {
  var progress = _ImportProgress.init;
  ImportResult? _importRes;
  var selectedDir = '';

  Future<void> _selectDir() async {
    final dir = await FilePicker.platform
        .getDirectoryPath(dialogTitle: "Import directory");

    if (dir != null) {
      setState(() {
        selectedDir = dir;
      });
    }
  }

  Future<void> _import() async {
    if (selectedDir.isEmpty) {
      return;
    }

    setState(() {
      progress = _ImportProgress.pending;
      _importRes = null;
    });

    try {
      final appState = context.read<AppState>();
      final accEdit = context.read<AccEdit>();
      final res = await appState.native.importData(inDir: selectedDir);
      setState(() {
        progress = _ImportProgress.success;
        _importRes = res;
        selectedDir = '';
      });
      appState.timeline.refresh();
      accEdit.refresh();
    } catch (e) {
      logger.error('Import failed: $e');
      setState(() {
        progress = _ImportProgress.error;
      });
    }
  }

  Widget _importStats() {
    final res = _importRes;
    if (res == null) {
      return const Text('Imported.');
    }

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        // Imported
        Text('Imported: ${res.imported}'),
        // Duplicates
        if (res.duplicates.isNotEmpty)
          Text('Skipped duplicates: ${res.duplicates.length}'),
        if (res.duplicates.isNotEmpty)
          Expanded(
            child: ListView.builder(
              itemCount: res.duplicates.length,
              itemBuilder: (context, index) => Text(
                '- ${res.duplicates[index]}',
                style: TextStyle(fontSize: 12, color: Colors.grey[600]),
              ),
            ),
          ),
        // Failed
        if (res.failed.isNotEmpty) Text('Failed: ${res.failed.length}'),
        if (res.failed.isNotEmpty)
          Expanded(
            child: ListView.builder(
              itemCount: res.failed.length,
              itemBuilder: (context, index) => Text(
                '- ${res.failed[index]}',
                style: TextStyle(fontSize: 12, color: Colors.grey[600]),
              ),
            ),
          ),
      ],
    );
  }

  @override
  Widget build(BuildContext context) {
    final importAllowed =
        selectedDir.isNotEmpty && progress != _ImportProgress.pending;
    final theme = Theme.of(context);
    final canOpenFinder =
        Platform.isLinux || Platform.isMacOS || Platform.isWindows;

    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        const Text(
            'Choose a directory where Bolik Timeline should copy the data from.'),
        const SizedBox(height: 18),
        OutlinedButton(
          onPressed: _selectDir,
          child: const Text("Choose directory"),
        ),
        const SizedBox(height: 8),
        if (selectedDir.isNotEmpty)
          Wrap(children: [
            const Text('Selected directory: '),

            // On mobiles just show the directory
            if (!canOpenFinder)
              Text(selectedDir, style: theme.textTheme.labelSmall),

            // On desktops allow to open the directory
            if (canOpenFinder)
              TextButton(
                onPressed: () {
                  launchUrl(Uri.parse('file:$selectedDir'));
                },
                child: Text(selectedDir, style: theme.textTheme.labelSmall),
              ),
          ]),
        const Spacer(),
        if (progress != _ImportProgress.success)
          ElevatedButton(
            onPressed: importAllowed ? _import : null,
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                const Text('Import'),
                if (progress == _ImportProgress.pending)
                  const SizedBox(width: 16),
                if (progress == _ImportProgress.pending)
                  SizedBox(
                    width: 16,
                    height: 16,
                    child: const CircularProgressIndicator(strokeWidth: 2),
                  )
              ],
            ),
          ),
        if (progress == _ImportProgress.success)
          Expanded(child: _importStats()),
        if (progress == _ImportProgress.error)
          const Text('There was an error...'),
        const SafeArea(child: SizedBox(height: 16)),
      ],
    );
  }
}

enum _ImportProgress {
  init,
  pending,
  error,
  success,
}
