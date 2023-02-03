import 'dart:io';

import 'package:file_picker/file_picker.dart';
import 'package:flutter/material.dart';
import 'package:intl/intl.dart';
import 'package:path/path.dart' as p;
import 'package:path_provider/path_provider.dart';
import 'package:provider/provider.dart';
import 'package:timeline/common/models/app_state.dart';
import 'package:timeline/common/models/phase_info.dart';
import 'package:url_launcher/url_launcher.dart';

class ExportPage extends StatelessWidget {
  const ExportPage({super.key});

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Export data'),
      ),
      body: Center(
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 16),
          constraints: const BoxConstraints(maxWidth: 500),
          child: const _ExportContent(),
        ),
      ),
    );
  }
}

// Currently, only desktop platforms are supported.
// Android doesn't make it easy nowadays to write to a user selected directory.
//
// Steps to make it work on Android:
// 1. Create Android platform integration: https://docs.flutter.dev/development/platform-integration/platform-channels
// 2. In Kotlin:
//    - Open document tree: https://developer.android.com/training/data-storage/shared/documents-files#grant-access-directory
//    - Specify permissions:
//        Intent.FLAG_GRANT_PERSISTABLE_URI_PERMISSION   <-- optional
//          | Intent.FLAG_GRANT_READ_URI_PERMISSION
//          | Intent.FLAG_GRANT_WRITE_URI_PERMISSION
//    - Access selected dir from `onActivityResult`
// 3. It seems that I cannot create nor write to files from Rust but it is possible to do from Kotlin :/
//    Examples:
//      val dir = DocumentFile.fromTreeUri(context, uri)!!
//      val file = dir.createFile("text/plain", "hello.txt")!!
//      contentResolver.openFileDescriptor(file.uri, "w").use { fd ->
//          FileOutputStream(fd!!.fileDescriptor).use { os ->
//              os.write("Hello from Android".encodeToByteArray())
//              os.flush()
//          }
//      }
// 4. I either need to figure out how to write from Rust or implement export in Kotlin...
//
// Android notes:
// - `onActivityResult` returns a virtual path. We need to convert it to real path to be able to use from Rust.
//    You may use `FileUtils.getFullPathFromTreeUri` from flutter file_picker plugin.
// - If you decided to grant persistable access then we need to persist the permissions in `onActivityResult`
//   with `contentResolver.takePersistableUriPermission(uri, Intent.FLAG_GRANT_WRITE_URI_PERMISSION or Intent.FLAG_GRANT_READ_URI_PERMISSION)`
// - It is potentially possible to start with Documents directory in the folder picker with
//   `putExtra(DocumentsContract.EXTRA_INITIAL_URI, Environment.DIRECTORY_DOCUMENTS)`
//   when launching the Intent.
//
// A good example: https://github.com/miguelpruivo/flutter_file_picker/issues/721#issuecomment-900100134
class _ExportContent extends StatefulWidget {
  const _ExportContent({super.key});

  @override
  State<StatefulWidget> createState() => _ExportContentState();
}

class _ExportContentState extends State<_ExportContent> {
  var progress = _ExportProgress.init;
  var selectedDir = '';

  @override
  void initState() {
    super.initState();
    _setup();
  }

  void _setup() async {
    try {
      if (canChooseDir) {
        // Let user choose directory
        return;
      }

      // Otherwise preselect a directory.
      // On iOS this will create a directory "Bolik Timeline" in Files. All files will be deleted when app is uninstalled.
      final docDir = await getApplicationDocumentsDirectory();
      setState(() {
        selectedDir = docDir.path;
      });
    } catch (e, st) {
      logger.error('Failed to auto select export directory: $e $st');
      setState(() {
        progress = _ExportProgress.error;
      });
    }
  }

  bool get canChooseDir => !Platform.isIOS;

  Future<void> _selectDir() async {
    final dir = await FilePicker.platform
        .getDirectoryPath(dialogTitle: "Export directory");

    if (dir != null) {
      setState(() {
        selectedDir = dir;
      });
    }
  }

  Future<void> _export() async {
    if (selectedDir.isEmpty) {
      return;
    }

    setState(() {
      progress = _ExportProgress.pending;
    });

    final appState = context.read<AppState>();
    try {
      await appState.native.exportData(outDir: selectedDir);
      setState(() {
        progress = _ExportProgress.success;
      });
    } catch (e) {
      logger.error('Export failed: $e');
      setState(() {
        progress = _ExportProgress.error;
      });
    }
  }

  @override
  Widget build(BuildContext context) {
    final exportAllowed =
        selectedDir.isNotEmpty && progress != _ExportProgress.pending;
    final theme = Theme.of(context);
    final canOpenFinder =
        Platform.isLinux || Platform.isMacOS || Platform.isWindows;

    final date = DateFormat('yyyy-MM-dd', 'en_US').format(DateTime.now());
    final dirNamePreview = selectedDir.isNotEmpty
        ? p.join(selectedDir, "Bolik Timeline export $date")
        : null;

    return Column(
      mainAxisSize: MainAxisSize.min,
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        if (canChooseDir)
          const Text(
              'Choose a directory where Bolik Timeline data should be saved to.'),
        const SizedBox(height: 18),
        if (canChooseDir)
          OutlinedButton(
            onPressed: _selectDir,
            child: const Text("Choose directory"),
          ),
        const SizedBox(height: 8),
        if (dirNamePreview != null && canChooseDir)
          Wrap(children: [
            const Text('Will export to: '),

            // On mobiles just show the directory
            if (!canOpenFinder)
              Text(dirNamePreview, style: theme.textTheme.labelSmall),

            // On desktops allow to open the directory
            if (canOpenFinder)
              TextButton(
                onPressed: () {
                  launchUrl(Uri.parse('file:$selectedDir'));
                },
                child: Text(dirNamePreview, style: theme.textTheme.labelSmall),
              ),
          ]),
        if (!canChooseDir)
          Padding(
            padding: const EdgeInsets.symmetric(vertical: 16, horizontal: 0),
            child: Column(
              crossAxisAlignment: CrossAxisAlignment.start,
              children: [
                Text(
                  '- You can preview exported files in "Bolik Timeline" folder in Files app.',
                  style: TextStyle(color: Colors.grey[800]),
                ),
                const SizedBox(height: 8),
                Text.rich(TextSpan(
                  children: [
                    TextSpan(
                      text:
                          '- Exported files will be deleted automatically when you uninstall the app.   ',
                      style: TextStyle(color: Colors.grey[800]),
                    ),
                    WidgetSpan(
                      child: Tooltip(
                        message:
                            'If you want to keep your backed up files even after uninstalling the app then copy the exported folder outside "Bolik Timeline" folder.',
                        triggerMode: TooltipTriggerMode.tap,
                        showDuration: const Duration(seconds: 15),
                        margin: const EdgeInsets.symmetric(horizontal: 16),
                        child: const Text(
                          'More info...',
                          style: TextStyle(color: Colors.blue),
                        ),
                      ),
                    ),
                  ],
                )),
              ],
            ),
          ),
        const Spacer(),
        if (progress != _ExportProgress.success)
          ElevatedButton(
            onPressed: exportAllowed ? _export : null,
            child: Row(
              mainAxisSize: MainAxisSize.min,
              children: [
                const Text('Export'),
                if (progress == _ExportProgress.pending)
                  const SizedBox(width: 16),
                if (progress == _ExportProgress.pending)
                  SizedBox(
                    width: 16,
                    height: 16,
                    child: const CircularProgressIndicator(strokeWidth: 2),
                  )
              ],
            ),
          ),
        if (progress == _ExportProgress.success) const Text('Exported.'),
        if (progress == _ExportProgress.error)
          const Text('There was an error...'),
        const SafeArea(child: SizedBox(height: 16)),
      ],
    );
  }
}

enum _ExportProgress {
  init,
  pending,
  error,
  success,
}
