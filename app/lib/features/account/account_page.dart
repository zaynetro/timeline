import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:intl/intl.dart';
import 'package:provider/provider.dart';
import 'package:timeline/common/models/app_state.dart';
import 'package:timeline/common/models/phase_info.dart';
import 'package:timeline/routes.dart';

class AccountPage extends StatelessWidget {
  const AccountPage({Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        leading: IconButton(
          onPressed: () {
            BolikRoutes.goBack();
          },
          tooltip: 'Go back',
          icon: const Icon(Icons.arrow_back),
        ),
        title: const Text('My Account'),
      ),
      body: Container(
        color: Colors.grey[100],
        child: _AccountPageContent(),
      ),
    );
  }
}

class _AccountPageContent extends StatefulWidget {
  @override
  State<StatefulWidget> createState() => _AccountPageContentState();
}

class _AccountPageContentState extends State<_AccountPageContent> {
  @override
  void initState() {
    super.initState();

    final appState = context.read<AppState>();
    appState.syncBackend();
  }

  @override
  Widget build(BuildContext context) {
    final accEdit = context.watch<AccEdit>();
    final theme = Theme.of(context);

    return CustomScrollView(
      slivers: [
        // Account name
        SliverToBoxAdapter(
          child: Container(
            color: theme.colorScheme.background,
            padding:
                const EdgeInsets.only(top: 16, bottom: 16, left: 8, right: 8),
            child: Column(
                crossAxisAlignment: CrossAxisAlignment.stretch,
                children: [
                  Row(children: [
                    if (accEdit.view.name.isNotEmpty)
                      Text('Hello ${accEdit.view.name}!',
                          style: theme.textTheme.titleLarge),
                    if (accEdit.view.name.isEmpty)
                      Text('Hello!', style: theme.textTheme.titleLarge),
                    const SizedBox(width: 8),
                    IconButton(
                      onPressed: () {
                        Navigator.pushNamed(context, BolikRoutes.accEdit);
                      },
                      iconSize: 20,
                      color: Colors.grey[700],
                      tooltip: 'Edit account name',
                      icon: const Icon(Icons.edit),
                    ),
                  ]),
                  const SizedBox(height: 8),
                  Row(children: [
                    const Text('Account ID: ', style: TextStyle(fontSize: 14)),
                    Flexible(
                      child: SelectableText(
                        accEdit.view.id,
                        style: TextStyle(fontSize: 12, color: Colors.grey[600]),
                      ),
                    ),
                    IconButton(
                      onPressed: () {
                        Clipboard.setData(ClipboardData(text: accEdit.view.id));
                      },
                      iconSize: 18,
                      tooltip: 'Copy account ID to clipboard',
                      icon: const Icon(Icons.copy),
                    )
                  ]),
                ]),
          ),
        ),

        // Add device
        SliverPadding(
          padding: const EdgeInsets.only(top: 16, bottom: 8),
          sliver: SliverToBoxAdapter(
            child: Padding(
              padding: const EdgeInsets.symmetric(horizontal: 8),
              child: Row(
                  mainAxisAlignment: MainAxisAlignment.spaceBetween,
                  children: [
                    Text('Devices:', style: theme.textTheme.titleLarge),
                    ElevatedButton.icon(
                      onPressed: () {
                        Navigator.pushNamed(context, BolikRoutes.accDevicesAdd);
                      },
                      icon: const Icon(Icons.add),
                      label: const Text('Add device'),
                    ),
                  ]),
            ),
          ),
        ),

        // Account devices
        SliverPadding(
          padding: const EdgeInsets.symmetric(vertical: 16),
          sliver: _DevicesList(),
        ),

        // Misc header
        SliverPadding(
          padding: const EdgeInsets.only(top: 24, bottom: 0, right: 8, left: 8),
          sliver: SliverToBoxAdapter(
            child: Text('Other settings:', style: theme.textTheme.titleLarge),
          ),
        ),

        // Misc
        _MiscSettings(),
      ],
    );
  }
}

class _DevicesList extends StatefulWidget {
  @override
  State<StatefulWidget> createState() => _DevicesListState();
}

class _DevicesListState extends State<_DevicesList> {
  Set<String> activeDevices = {};

  @override
  void initState() {
    super.initState();
    _loadAccountGroup();
  }

  _loadAccountGroup() async {
    final appState = context.read<AppState>();
    final group = await appState.native.accountGroup();
    setState(() => activeDevices = group.devices.toSet());
  }

  @override
  Widget build(BuildContext context) {
    final appState = context.read<AppState>();
    final accEdit = context.watch<AccEdit>();
    final devices = accEdit.view.devices
      ..sort((a, b) => a.name.compareTo(b.name));
    final theme = Theme.of(context);
    final thisDeviceId = appState.getCurrentDeviceId();

    return SliverList(
      delegate: SliverChildBuilderDelegate(
        (context, index) {
          final device = devices[index];
          final isLast = (index + 1) == devices.length;
          final thisDevice = device.id == thisDeviceId;
          final addedAt = DateFormat.yMMMd().format(device.addedAt);
          final inactive = !activeDevices.contains(device.id);

          return Container(
            decoration: BoxDecoration(
              color: theme.colorScheme.background,
              border: isLast
                  ? null
                  : Border(
                      bottom: BorderSide(color: Colors.grey[400]!),
                    ),
            ),
            child: ListTile(
              leading: const Icon(Icons.computer),
              title: Row(children: [
                Text(device.name),
                const SizedBox(width: 8),
                if (thisDevice)
                  const Text(
                    '(This device)',
                    style: TextStyle(fontSize: 12, color: Colors.grey),
                  ),
              ]),
              subtitle: Row(children: [
                Text(
                  'Added $addedAt',
                  style: const TextStyle(fontSize: 12),
                ),
                const SizedBox(width: 8),
                if (inactive) const Text('⦁', style: TextStyle(fontSize: 12)),
                const SizedBox(width: 8),
                if (inactive)
                  const Text(
                    'Inactive',
                    style: TextStyle(fontSize: 12, color: Colors.orange),
                  ),
              ]),
            ),
          );
        },
        childCount: devices.length,
      ),
    );
  }
}

class _MiscSettings extends StatefulWidget {
  @override
  State<StatefulWidget> createState() => _MiscSettingsState();
}

class _MiscSettingsState extends State<_MiscSettings> {
  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    final canImport = Platform.isLinux ||
        Platform.isMacOS ||
        Platform.isWindows ||
        Platform.isIOS;
    final canExport = Platform.isLinux ||
        Platform.isMacOS ||
        Platform.isWindows ||
        Platform.isIOS;

    return SliverList(
      delegate: SliverChildListDelegate([
        ExpansionTile(
          title: const Text('Advanced'),
          backgroundColor: theme.colorScheme.background,
          children: [
            if (canImport)
              ListTile(
                leading: const Icon(Icons.import_export),
                title: const Text('Import data'),
                onTap: () {
                  Navigator.pushNamed(context, BolikRoutes.importData);
                },
              ),
            if (canExport)
              ListTile(
                // There is export_notes materia font icon but not in Flutter...
                leading: const Icon(Icons.arrow_upward),
                title: const Text('Export data'),
                onTap: () {
                  Navigator.pushNamed(context, BolikRoutes.exportData);
                },
              ),
            ListTile(
              onTap: () {
                // We replace current page so that Dialog will be closed on desktops
                BolikRoutes.rootNav.currentState!
                    .pushReplacementNamed(BolikRoutes.logs);
              },
              leading: const Icon(Icons.engineering),
              title: const Text("Logs"),
            ),
          ],
        ),
        ListTile(
          tileColor: theme.colorScheme.background,
          title: Text(
            'Log out',
            style: TextStyle(color: theme.colorScheme.error),
          ),
          onTap: () {
            Navigator.pushNamed(context, BolikRoutes.logout);
          },
        )
      ]),
    );
  }
}

class EditAccountPage extends StatefulWidget {
  const EditAccountPage({super.key});

  @override
  State<StatefulWidget> createState() => _EditAccountPageState();
}

class _EditAccountPageState extends State<EditAccountPage> {
  late final TextEditingController _nameController;
  String? _error;
  bool _saving = false;

  @override
  void initState() {
    super.initState();

    final accEdit = context.read<AccEdit>();
    _nameController = TextEditingController(text: accEdit.view.name);
  }

  @override
  void dispose() {
    _nameController.dispose();
    super.dispose();
  }

  void _onSave() async {
    if (_saving) {
      return;
    }

    setState(() {
      _error = null;
      _saving = true;
    });

    final name = _nameController.text;
    final accEdit = context.read<AccEdit>();
    if (name == accEdit.view.name) {
      Navigator.pop(context);
      return;
    }

    try {
      await accEdit.editName(name);

      if (mounted) {
        Navigator.pop(context);
      }
    } catch (e) {
      logger.error("Failed to edit name: $e");
      setState(() => _error = 'Failed to edit name.');
    } finally {
      setState(() => _saving = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(
        leading: IconButton(
          onPressed: () {
            Navigator.pop(context);
          },
          tooltip: 'Cancel',
          icon: const Icon(Icons.close),
        ),
        title: const Text('Edit account'),
        actions: [
          TextButton.icon(
            onPressed: _saving
                ? null
                : () {
                    _onSave();
                  },
            icon: _saving
                ? const SizedBox(
                    width: 20,
                    height: 20,
                    child: CircularProgressIndicator(strokeWidth: 3),
                  )
                : const Icon(Icons.done),
            label: const Text('Save'),
          ),
        ],
      ),
      body: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 32, vertical: 8),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            TextField(
              controller: _nameController,
              decoration: const InputDecoration(
                labelText: 'Name',
                helperMaxLines: 3,
                helperText: _nameHelperText,
              ),
              textCapitalization: TextCapitalization.words,
            ),
            if (_error != null)
              Padding(
                padding: const EdgeInsets.only(top: 32),
                child: Text(
                  _error!,
                  style: TextStyle(color: theme.colorScheme.error),
                ),
              )
          ],
        ),
      ),
    );
  }
}

const _nameHelperText =
    """Account name is encrypted and shared only with your contacts.""";

class LogOutPage extends StatelessWidget {
  const LogOutPage({super.key});

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(
        title: const Text('Log out'),
      ),
      body: Padding(
        padding: const EdgeInsets.symmetric(vertical: 16, horizontal: 32),
        child: Column(
          children: [
            Text(
              'Are you sure you want to log out?',
              style: theme.textTheme.headlineMedium,
            ),
            const SizedBox(height: 32),
            const Text(
                'If you lose access to all of your devices you will not be able to log back in.'),
            const SizedBox(height: 16),
            TextButton(
              onPressed: () {
                final appState = context.read<AppState>();
                appState.dangerousLogout();
              },
              child: Text(
                'Yes, log me out',
                style: TextStyle(color: theme.colorScheme.error),
              ),
            )
          ],
        ),
      ),
    );
  }
}
