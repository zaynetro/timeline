import 'dart:async';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:provider/provider.dart';
import 'package:qr_flutter/qr_flutter.dart';
import 'package:timeline/common/models/pre_account_state.dart';
import 'package:timeline/routes.dart';

class AccountSetupPage extends StatelessWidget {
  const AccountSetupPage({Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    final state = context.read<PreAccountState>();

    return Scaffold(
      body: _AccountSetupContent(state),
    );
  }
}

class _AccountSetupContent extends StatefulWidget {
  final PreAccountState state;
  const _AccountSetupContent(this.state, {Key? key}) : super(key: key);

  @override
  State<StatefulWidget> createState() => _AccountSetupContentState();
}

enum _SetupPhase {
  initial,
  inputAccountName,
  createAccount,
  connectToAccount,
}

class _AccountSetupContentState extends State<_AccountSetupContent> {
  var _phase = _SetupPhase.initial;

  @override
  Widget build(BuildContext context) {
    if (_phase == _SetupPhase.createAccount) {
      return _AccountCreation();
    } else if (_phase == _SetupPhase.connectToAccount) {
      return _ConnectToAccount(widget.state, onCancel: () {
        setState(() {
          _phase = _SetupPhase.initial;
        });
      });
    } else if (_phase == _SetupPhase.inputAccountName) {
      return _AccountNameContent(
        onSubmit: (name) {
          setState(() {
            _phase = _SetupPhase.createAccount;
          });
          widget.state.createAccount(name);
        },
      );
    }

    final textTheme = Theme.of(context).textTheme;

    return Center(
      child: Column(
        children: [
          const SizedBox(height: 80),
          Text(
            'Welcome to Bolik Timeline!',
            style: textTheme.displaySmall,
            textAlign: TextAlign.center,
          ),
          const SizedBox(height: 50),
          ElevatedButton.icon(
            onPressed: () {
              setState(() {
                _phase = _SetupPhase.inputAccountName;
              });
            },
            icon: const Icon(Icons.create),
            label: const Text('Create new account'),
          ),
          const SizedBox(height: 30),
          ElevatedButton.icon(
            onPressed: () {
              setState(() {
                _phase = _SetupPhase.connectToAccount;
              });
            },
            icon: const Icon(Icons.link),
            label: const Text('Connect to existing account'),
          ),
        ],
      ),
    );
  }
}

class _AccountNameContent extends StatefulWidget {
  final void Function(String) onSubmit;

  const _AccountNameContent({super.key, required this.onSubmit});

  @override
  State<StatefulWidget> createState() => _AccountNameContentState();
}

class _AccountNameContentState extends State<_AccountNameContent> {
  final controller = TextEditingController();

  @override
  void dispose() {
    controller.dispose();
    super.dispose();
  }

  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        children: [
          const SizedBox(height: 100),
          const Text('What is your name?'),
          ConstrainedBox(
            constraints: const BoxConstraints(maxWidth: 300),
            child: TextField(
              controller: controller,
              onSubmitted: (name) => widget.onSubmit(name),
              decoration: const InputDecoration(
                labelText: 'Optional name',
                helperText: _nameHelperText,
                helperMaxLines: 5,
              ),
              textCapitalization: TextCapitalization.words,
            ),
          ),
          const SizedBox(height: 20),
          ElevatedButton(
            onPressed: () => widget.onSubmit(controller.text),
            child: const Text('Continue'),
          ),
        ],
      ),
    );
  }
}

const _nameHelperText =
    """Account name is encrypted and shared only with your contacts.""";

class _AccountCreation extends StatelessWidget {
  @override
  Widget build(BuildContext context) {
    return Center(
      child: Column(
        children: const [
          SizedBox(height: 100),
          Text('Creating an account for you...'),
        ],
      ),
    );
  }
}

class _ConnectToAccount extends StatefulWidget {
  final PreAccountState state;
  final Function() onCancel;

  const _ConnectToAccount(this.state, {required this.onCancel});

  @override
  State<StatefulWidget> createState() => _ConnectToAccountState();
}

class _ConnectToAccountState extends State<_ConnectToAccount> {
  var _thisDeviceShare = '';
  var _shareVariant = _ShareVariant.qr;
  Timer? _timer;

  @override
  void initState() {
    super.initState();
    _setupShare();

    _timer = Timer.periodic(const Duration(seconds: 3), (_) {
      widget.state.syncBackend();
    });
  }

  @override
  void dispose() {
    _timer?.cancel();
    super.dispose();
  }

  _setupShare() async {
    final share = await widget.state.getDeviceShare();
    setState(() {
      _thisDeviceShare = share;
    });
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Connect to existing account'),
        centerTitle: true,
        actions: [
          PopupMenuButton<String>(
            tooltip: 'Options',
            onSelected: (action) {
              if (action == 'logs') {
                Navigator.pushNamed(context, BolikRoutes.logs);
              }
            },
            itemBuilder: (context) => [
              const PopupMenuItem(
                value: 'logs',
                child: Text('Logs'),
              ),
            ],
          ),
        ],
      ),
      body: Center(
        child: Container(
          padding: const EdgeInsets.symmetric(horizontal: 8, vertical: 20),
          width: 400,
          // constraints: const BoxConstraints(maxWidth: 500),
          child: _shareVariant == _ShareVariant.qr
              ? _QrCodeDisplay(
                  deviceShare: _thisDeviceShare,
                  toggleView: () =>
                      setState(() => _shareVariant = _ShareVariant.text),
                )
              : _ShareTextDisplay(
                  deviceShare: _thisDeviceShare,
                  toggleView: () =>
                      setState(() => _shareVariant = _ShareVariant.qr),
                ),
        ),
      ),
    );
  }
}

class _QrCodeDisplay extends StatelessWidget {
  final String deviceShare;
  final Function() toggleView;

  const _QrCodeDisplay({required this.deviceShare, required this.toggleView});

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Text('1. Open Bolik Timeline on your existing device'),
        const SizedBox(height: 8),
        const Text.rich(
          TextSpan(
            text: '2. Tap Menu ',
            children: [
              WidgetSpan(child: Icon(Icons.menu, size: 18)),
              TextSpan(text: ' and select "My Account"'),
            ],
          ),
        ),
        const SizedBox(height: 8),
        const Text('3. Tap on "Add device"'),
        const SizedBox(height: 8),
        const Text('4. Point camera to this screen to capture the code'),
        const SizedBox(height: 20),
        if (deviceShare.isNotEmpty)
          Center(
            child: QrImage(
              data: deviceShare,
              version: QrVersions.auto,
              size: 300,
              gapless: true,
              errorCorrectionLevel: QrErrorCorrectLevel.M,
              semanticsLabel: 'This device share',
            ),
          ),
        const Spacer(),
        TextButton(
          onPressed: toggleView,
          child: const Text("Existing device doesn't have a camera"),
        ),
      ],
    );
  }
}

class _ShareTextDisplay extends StatelessWidget {
  final String deviceShare;
  final Function() toggleView;

  const _ShareTextDisplay(
      {required this.deviceShare, required this.toggleView});

  @override
  Widget build(BuildContext context) {
    final textTheme = Theme.of(context).textTheme;

    return Column(
      crossAxisAlignment: CrossAxisAlignment.start,
      children: [
        const Text('1. Open Bolik Timeline on your existing device'),
        const SizedBox(height: 8),
        const Text.rich(
          TextSpan(
            text: '2. Tap Menu ',
            children: [
              WidgetSpan(child: Icon(Icons.menu, size: 18)),
              TextSpan(text: ' and select "My Account"'),
            ],
          ),
        ),
        const SizedBox(height: 8),
        const Text('3. Tap on "Add device"'),
        const SizedBox(height: 8),
        const Text('4. Enter the code below into "Device share" text field'),
        const SizedBox(height: 32),
        Row(children: [
          Text('This device share:', style: textTheme.headlineSmall),
          const SizedBox(width: 8),
          IconButton(
            onPressed: () {
              Clipboard.setData(ClipboardData(text: deviceShare));
            },
            tooltip: 'Copy to clipboard',
            icon: const Icon(Icons.copy),
          )
        ]),
        const SizedBox(height: 20),
        SelectableText(
          deviceShare,
          style: TextStyle(color: Colors.grey[800], fontSize: 12),
        ),
        const Spacer(),
        TextButton(
          onPressed: toggleView,
          child: const Text("Existing device has a camera"),
        ),
      ],
    );
  }
}

enum _ShareVariant {
  qr,
  text,
}
