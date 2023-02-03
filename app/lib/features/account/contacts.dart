import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:provider/provider.dart';
import 'package:timeline/bridge_generated.dart';
import 'package:timeline/common/models/app_state.dart';
import 'package:timeline/common/models/phase_info.dart';
import 'package:timeline/routes.dart';

class ContactsListArgs {
  final void Function(AccContact contact)? onSelect;

  ContactsListArgs({this.onSelect});
}

class ContactsListPage extends StatefulWidget {
  final ContactsListArgs args;
  const ContactsListPage({super.key, required this.args});

  @override
  State<StatefulWidget> createState() => _ContactsListPageState();
}

class _ContactsListPageState extends State<ContactsListPage> {
  final _textController = TextEditingController();

  @override
  void dispose() {
    _textController.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    final accEdit = context.watch<AccEdit>();

    final contacts = accEdit.view.contacts;
    final text = _textController.text;
    final textFilter = text.toLowerCase();

    contacts.sort((a, b) => a.name.compareTo(b.name));
    final filteredContacts = contacts
        .where((c) => c.name.toLowerCase().contains(textFilter))
        .toList();

    final theme = Theme.of(context);

    return Scaffold(
      appBar: AppBar(
        leading: IconButton(
          onPressed: () {
            BolikRoutes.goBack();
          },
          tooltip: 'Go back',
          icon: const Icon(Icons.arrow_back),
        ),
        title: TextField(
          controller: _textController,
          decoration: const InputDecoration(
            hintText: 'Filter contacts',
          ),
          onChanged: (content) {
            // Rebuild the widget to filter list items
            setState(() {});
          },
        ),
      ),
      body: Column(
        children: [
          const SizedBox(height: 8),
          ListTile(
            onTap: () {
              Navigator.pushNamed(context, BolikRoutes.accContactsAdd);
            },
            leading: Icon(Icons.add, color: theme.colorScheme.primary),
            title: Text(
              'Add new contact',
              style: TextStyle(color: theme.colorScheme.primary),
            ),
            hoverColor: theme.colorScheme.primary.withOpacity(0.08),
          ),
          const SizedBox(height: 8),
          Expanded(
            child: ListView.builder(
              itemBuilder: (context, index) {
                final c = filteredContacts[index];
                return ListTile(
                  leading: const Icon(Icons.person),
                  title: Text(c.name),
                  trailing: widget.args.onSelect != null
                      ? null
                      : const Icon(Icons.edit),
                  onTap: () {
                    if (widget.args.onSelect != null) {
                      widget.args.onSelect!(c);
                    } else {
                      Navigator.pushNamed(context, BolikRoutes.accContactsEdit,
                          arguments: EditContactArgs(c));
                    }
                  },
                );
              },
              itemCount: filteredContacts.length,
            ),
          ),
        ],
      ),
    );
  }
}

class AddContactPage extends StatefulWidget {
  const AddContactPage({super.key});

  @override
  State<StatefulWidget> createState() => _AddContactPageState();
}

class _AddContactPageState extends State<AddContactPage> {
  final _idController = TextEditingController();
  final _nameController = TextEditingController();
  final _formKey = GlobalKey<FormState>();
  String? _error;
  bool _creating = false;

  @override
  void dispose() {
    _idController.dispose();
    _nameController.dispose();
    super.dispose();
  }

  void _onAdd() async {
    if (_creating) {
      return;
    }

    setState(() => _error = null);
    if (!_formKey.currentState!.validate()) {
      return;
    }

    setState(() => _creating = true);
    final accEdit = context.read<AccEdit>();
    final id = _idController.text;
    final name = _nameController.text;
    try {
      await accEdit.addContact(accountId: id, name: name);

      if (mounted) {
        Navigator.pop(context);
      }
    } catch (e) {
      logger.error("Failed to add a contact: $e");
      setState(() => _error = 'Failed to add a contact.');
    } finally {
      setState(() => _creating = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final accEdit = context.read<AccEdit>();
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
        title: const Text('Add new contact'),
        actions: [
          TextButton.icon(
            onPressed: _creating
                ? null
                : () {
                    _onAdd();
                  },
            icon: _creating
                ? const SizedBox(
                    width: 20,
                    height: 20,
                    child: CircularProgressIndicator(strokeWidth: 3),
                  )
                : const Icon(Icons.done),
            label: const Text('Add'),
          ),
        ],
      ),
      body: Form(
        key: _formKey,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 32, vertical: 8),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              TextFormField(
                controller: _nameController,
                decoration: const InputDecoration(
                  labelText: 'Name',
                ),
                validator: (name) {
                  if (name == null || name.trim().isEmpty) {
                    return 'Please specify a name';
                  }
                  return null;
                },
                textCapitalization: TextCapitalization.words,
              ),
              const SizedBox(height: 8),
              TextFormField(
                controller: _idController,
                decoration: const InputDecoration(
                  labelText: 'Account ID',
                  helperMaxLines: 5,
                  helperText: _idHelperText,
                ),
                validator: (id) {
                  if (id == null || id.trim().isEmpty) {
                    return 'Please specify an ID';
                  } else if (id == accEdit.view.id) {
                    return 'Specify an ID of a contact you want to add';
                  }

                  return null;
                },
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
      ),
    );
  }
}

const _idHelperText = """How to find Account ID?

1. Tap Menu  ☰  and select "My Account"
2. Account ID is displayed under account name""";

class EditContactArgs {
  final AccContact contact;

  EditContactArgs(this.contact);
}

class EditContactPage extends StatefulWidget {
  final EditContactArgs args;
  const EditContactPage({super.key, required this.args});

  @override
  State<StatefulWidget> createState() => _EditContactPageState();
}

class _EditContactPageState extends State<EditContactPage> {
  final _nameController = TextEditingController();
  final _formKey = GlobalKey<FormState>();
  String? _error;
  bool _saving = false;

  @override
  void initState() {
    super.initState();

    _nameController.text = widget.args.contact.name;
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

    setState(() => _error = null);
    if (!_formKey.currentState!.validate()) {
      return;
    }

    final name = _nameController.text;
    if (name == widget.args.contact.name) {
      Navigator.pop(context);
      return;
    }

    setState(() => _saving = true);
    try {
      final accEdit = context.read<AccEdit>();
      await accEdit.editContactName(
          accountId: widget.args.contact.accountId, name: name);

      if (mounted) {
        Navigator.pop(context);
      }
    } catch (e) {
      logger.error("Failed to edit a contact: $e");
      setState(() => _error = 'Failed to edit a contact.');
    } finally {
      setState(() => _saving = false);
    }
  }

  @override
  Widget build(BuildContext context) {
    final theme = Theme.of(context);
    final contactId = widget.args.contact.accountId;

    return Scaffold(
      appBar: AppBar(
        leading: IconButton(
          onPressed: () {
            Navigator.pop(context);
          },
          tooltip: 'Cancel',
          icon: const Icon(Icons.close),
        ),
        title: const Text('Edit contact'),
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
      body: Form(
        key: _formKey,
        child: Padding(
          padding: const EdgeInsets.symmetric(horizontal: 32, vertical: 8),
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.stretch,
            children: [
              TextFormField(
                controller: _nameController,
                decoration: const InputDecoration(
                  labelText: 'Name',
                ),
                validator: (name) {
                  if (name == null || name.trim().isEmpty) {
                    return 'Please specify a name';
                  }
                  return null;
                },
                textCapitalization: TextCapitalization.words,
              ),
              const SizedBox(height: 16),
              Row(children: [
                const Text('Account ID: ', style: TextStyle(fontSize: 14)),
                Flexible(
                  child: SelectableText(
                    contactId,
                    style: TextStyle(fontSize: 12, color: Colors.grey[600]),
                  ),
                ),
                IconButton(
                  onPressed: () {
                    Clipboard.setData(ClipboardData(text: contactId));
                  },
                  iconSize: 18,
                  tooltip: 'Copy account ID to clipboard',
                  icon: const Icon(Icons.copy),
                )
              ]),
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
      ),
    );
  }
}
