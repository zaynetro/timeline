import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:timeline/bridge_generated.dart';
import 'package:timeline/common/models/app_state.dart';
import 'package:timeline/common/models/phase_info.dart';
import 'package:timeline/features/account/contacts.dart';
import 'package:timeline/features/card/card_edit.dart';
import 'package:timeline/routes.dart';

class CollaboratorsPage extends StatefulWidget {
  const CollaboratorsPage({super.key});

  @override
  State<StatefulWidget> createState() => _CollaboratorsPageState();
}

class _CollaboratorsPageState extends State<CollaboratorsPage> {
  final _textController = TextEditingController();
  final Map<String, ProfileView> _profiles = {};
  final Map<String, AclRights?> _pendingChanges = {};
  late final TimelineCardEdit cardEdit;
  late final AclEntry ownRights;

  bool _saving = false;
  String? _error;

  @override
  void initState() {
    super.initState();
    cardEdit = context.read<TimelineCardEdit>();
    final accEdit = context.read<AccEdit>();

    for (var entry in cardEdit.card.acl.accounts) {
      if (entry.accountId == accEdit.view.id) {
        ownRights = AclEntry(accountId: entry.accountId, rights: entry.rights);
        break;
      }
    }
    _load();
  }

  void _load() async {
    try {
      final appState = context.read<AppState>();
      final list = await appState.native.listProfiles();
      _profiles.clear();
      for (var profile in list) {
        _profiles[profile.accountId] = profile;
      }
      setState(() {});
    } catch (e) {
      logger.warn("Failed to list profiles: $e");
    }
  }

  @override
  void dispose() {
    _textController.dispose();
    super.dispose();
  }

  void _onSave() async {
    if (_saving) {
      return;
    }

    // Apply pending changes
    if (ownRights.rights == AclRights.Admin) {
      final changes = _pendingChanges.entries
          .map((e) => CollaboratorChange(
              accountId: e.key,
              rights: e.value ?? AclRights.Read,
              removed: e.value == null))
          .toList();

      setState(() {
        _error = null;
        _saving = true;
      });
      try {
        await cardEdit.editCollaborators(changes);
      } catch (e) {
        logger.error("Failed to edit collaborators: $e");
        setState(() => _error = 'Failed to edit collaborators.');
        return;
      } finally {
        setState(() => _saving = false);
      }
    }

    if (mounted) {
      BolikRoutes.goBack();
    }
  }

  @override
  Widget build(BuildContext context) {
    final accEdit = context.read<AccEdit>();
    final acc = accEdit.view;
    final contacts = acc.contacts;
    final contactsMap = {for (var c in contacts) c.accountId: c};

    String findAclName(String accountId) =>
        contactsMap[accountId]?.name ??
        _profiles[accountId]?.name ??
        'Unknown #${accountId.substring(0, 6)}';

    // Merge ACL entries
    final Map<String, _AclRow> aclMap = {};

    // Saved ACL
    for (var e in cardEdit.card.acl.accounts) {
      if (e.accountId == acc.id) {
        // We handle own account separately
        continue;
      }

      final name = findAclName(e.accountId);
      aclMap[e.accountId] = _AclRow(
        name: name,
        accountId: e.accountId,
        rights: e.rights,
        isContact: contactsMap.containsKey(e.accountId),
      );
    }

    // Pending ACL changes
    for (var e in _pendingChanges.entries) {
      if (e.value == null) {
        aclMap.remove(e.key);
        continue;
      }

      final name = findAclName(e.key);
      aclMap[e.key] = _AclRow(
        name: name,
        accountId: e.key,
        rights: e.value!,
        isContact: contactsMap.containsKey(e.key),
      );
    }

    final aclRows = aclMap.values.toList();

    // Sort ACL rows and add own account row as first
    aclRows.sort((a, b) => a.name.compareTo(b.name));
    aclRows.insert(
        0,
        _AclRow(
          name: '${acc.name} (you)',
          accountId: acc.id,
          rights: ownRights.rights,
          isContact: true,
        ));

    return Scaffold(
      appBar: AppBar(
        leading: IconButton(
          onPressed: () {
            BolikRoutes.goBack();
          },
          tooltip: 'Go back',
          icon: const Icon(Icons.close),
        ),
        title: const Text('Edit collaborators'),
        actions: [
          Tooltip(
            triggerMode: TooltipTriggerMode.tap,
            message: _aclRightsHelp(ownRights.rights),
            child: Icon(Icons.help_outline, color: Colors.grey[700]),
          ),
          const SizedBox(width: 8),
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
      body: _AclList(
        acc: accEdit.view,
        acl: aclRows,
        error: _error,
        ownRights: ownRights,
        onContactMode: () {
          Navigator.pushNamed(
            context,
            BolikRoutes.accContacts,
            arguments: ContactsListArgs(onSelect: (c) {
              Navigator.pop(context);
              _pendingChanges[c.accountId] = AclRights.Read;
              setState(() {});
            }),
          );
        },
        onChange: (String accountId, AclRights? rights) {
          _pendingChanges[accountId] = rights;
          setState(() {});
        },
      ),
    );
  }
}

class _AclList extends StatelessWidget {
  final List<_AclRow> acl;
  final AclEntry ownRights;
  final AccView acc;
  final String? error;
  final Function() onContactMode;
  final Function(String accountId, AclRights? rights) onChange;

  const _AclList({
    super.key,
    required this.acl,
    required this.ownRights,
    required this.acc,
    required this.error,
    required this.onContactMode,
    required this.onChange,
  });

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final isAdmin = ownRights.rights == AclRights.Admin;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        // Admins can add more collaborators
        const SizedBox(height: 8),
        if (isAdmin)
          ListTile(
            leading: Icon(Icons.add, color: theme.colorScheme.primary),
            title: Text(
              'Select from contacts',
              style: TextStyle(color: theme.colorScheme.primary),
            ),
            hoverColor: theme.colorScheme.primary.withOpacity(0.08),
            onTap: onContactMode,
          ),
        if (isAdmin) const SizedBox(height: 8),

        // Error
        if (error != null)
          Padding(
            padding: const EdgeInsets.only(bottom: 8),
            child: Text(
              error!,
              style: TextStyle(color: theme.colorScheme.error),
            ),
          ),

        // List of collaborators
        Flexible(
          child: ListView.builder(
            itemBuilder: (context, index) {
              final entry = acl[index];
              final name = entry.name;
              final disabled =
                  !isAdmin || entry.accountId == ownRights.accountId;

              return ListTile(
                leading: entry.isContact
                    ? const Icon(Icons.person)
                    : const Icon(Icons.person_off_outlined),
                title: Text(name, maxLines: 1),
                onTap: () {},
                trailing: DropdownButton<AclRights>(
                  items: [
                    _buildDropdownItem(AclRights.Read),
                    _buildDropdownItem(AclRights.Write),
                    _buildDropdownItem(AclRights.Admin),
                    // NOTE: Sadly, flutter doesn't allow changing the height of a menu item
                    //       to smaller values. In the future I can replace DropdownButton with
                    //       PopupMenuButton.
                    const DropdownMenuItem(
                      enabled: false,
                      child: SizedBox(),
                    ),
                    DropdownMenuItem(
                      child: Text(
                        'Remove access',
                        style: TextStyle(color: theme.colorScheme.error),
                      ),
                      onTap: () {
                        ScaffoldMessenger.of(context).showSnackBar(
                            const SnackBar(content: Text('Not implemented')));

                        // onChange(entry.accountId, null);
                      },
                    ),
                  ],
                  onChanged: disabled
                      ? null
                      : (v) {
                          if (v != null && v != entry.rights) {
                            onChange(entry.accountId, v);
                          }
                        },
                  value: entry.rights,
                ),
              );
            },
            itemCount: acl.length,
          ),
        ),
        const SizedBox(height: 16),
      ],
    );
  }
}

String _aclRightsText(AclRights rights) {
  switch (rights) {
    case AclRights.Read:
      return 'Viewer';
    case AclRights.Write:
      return 'Editor';
    case AclRights.Admin:
      return 'Admin';
  }
}

String _aclRightsHelp(AclRights rights) {
  switch (rights) {
    case AclRights.Read:
      return _readRightsHelp;
    case AclRights.Write:
      return _writeRightsHelp;
    case AclRights.Admin:
      return _adminRightsHelp;
  }
}

const _readRightsHelp = """You have Read access.
You can view the card but you cannot make any changes.

Contact Admin if you need more access.""";

const _writeRightsHelp = """You have Edit access.
You can edit the card but you cannot edit the list of collaborators.

Contact Admin if you need more access.""";

const _adminRightsHelp = """You have Admin access.
You can edit the card and you can edit the list of collaborators.

Available permissions:
- Admin: User can edit the card and the list of collaborators.
- Edit: User can edit the card but cannot edit the list of collaborators.
- Read: User can view the card but cannot make any changes.""";

class _AclRow {
  final String name;
  final String accountId;
  final AclRights rights;
  final bool isContact;

  _AclRow({
    required this.name,
    required this.accountId,
    required this.rights,
    required this.isContact,
  });
}

DropdownMenuItem<AclRights> _buildDropdownItem(AclRights rights) {
  return DropdownMenuItem(
    value: rights,
    // Currently, we allow only one Admin
    enabled: rights != AclRights.Admin,
    child: Text(_aclRightsText(rights)),
  );
}
