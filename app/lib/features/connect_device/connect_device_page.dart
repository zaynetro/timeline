import 'dart:io';

import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import 'package:provider/provider.dart';
import 'package:timeline/bridge_generated.dart';
import 'package:timeline/common/models/app_state.dart';
import 'package:camera/camera.dart';
import 'package:timeline/common/models/phase_info.dart';

class ConnectDevicePage extends StatefulWidget {
  const ConnectDevicePage({super.key});

  @override
  State<StatefulWidget> createState() => _ConnectDevicePageState();
}

class _ConnectDevicePageState extends State<ConnectDevicePage> {
  var _qrScanSupported = Platform.isAndroid || Platform.isIOS;

  Future<void> _linkDevice(String share) async {
    final accEdit = context.read<AccEdit>();
    final linkedDeviceName = await accEdit.linkDevice(share);
    HapticFeedback.mediumImpact();

    if (mounted) {
      Navigator.pop(context);
      ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('Added device "$linkedDeviceName"')));
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        title: const Text('Link device'),
        centerTitle: true,
        actions: [
          if (_qrScanSupported)
            PopupMenuButton<String>(
              tooltip: 'Options',
              onSelected: (action) {
                if (action == 'manual') {
                  setState(() {
                    _qrScanSupported = false;
                  });
                }
              },
              itemBuilder: (context) => [
                const PopupMenuItem(
                  value: 'manual',
                  child: Text('Enter code manually'),
                ),
              ],
            ),
        ],
      ),
      body: Padding(
        padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 16),
        child: _qrScanSupported
            ? _ConnectDeviceQrContent(linkDevice: _linkDevice)
            : _ConnectDeviceManualContent(linkDevice: _linkDevice),
      ),
    );
  }
}

class _ConnectDeviceManualContent extends StatefulWidget {
  final Function(String) linkDevice;
  const _ConnectDeviceManualContent({Key? key, required this.linkDevice})
      : super(key: key);

  @override
  State<StatefulWidget> createState() => _ConnectDeviceManualContentState();
}

class _ConnectDeviceManualContentState
    extends State<_ConnectDeviceManualContent> {
  String? _error;

  @override
  Widget build(BuildContext context) {
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 500),
        child: Column(
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            const Text('1. Open Bolik Timeline on new device'),
            const SizedBox(height: 8),
            const Text('2. Tap "Connect to existing device"'),
            const SizedBox(height: 8),
            const Text('3. Tap "Existing device doesn\'t have a camera"'),
            const SizedBox(height: 8),
            const Text('4. Enter the code into the text field below'),
            const SizedBox(height: 32),
            TextField(
              decoration: InputDecoration(
                labelText: 'Device share',
                errorText: _error,
              ),
              onChanged: (share) async {
                if (share.length < 20) {
                  return;
                }

                try {
                  await widget.linkDevice(share.trim());
                } catch (e) {
                  logger.error('Failed to link device: $e');
                  setState(() {
                    _error = 'Failed to link a device';
                  });
                }
              },
            )
          ],
        ),
      ),
    );
  }
}

class _ConnectDeviceQrContent extends StatefulWidget {
  final Function(String) linkDevice;

  const _ConnectDeviceQrContent({Key? key, required this.linkDevice})
      : super(key: key);

  @override
  State<StatefulWidget> createState() => _ConnectDeviceQrContentState();
}

class _ConnectDeviceQrContentState extends State<_ConnectDeviceQrContent> {
  String? _error;
  CameraController? controller;

  @override
  void initState() {
    super.initState();
    _setupCameras();
  }

  void _setupCameras() async {
    final state = context.read<AppState>();
    final cameras = await availableCameras();
    if (cameras.isEmpty) {
      setState(() {
        _error = 'No available cameras.';
      });
      return;
    }

    // I can specify ImageFormatGroup.jpeg (not supported on all devices) or ImageFormatGroup.yuv420.
    // For jpeg I can just feed RGB pixels directly into image.
    // For yuv I need to convert pixels to RGB color palette first.
    controller = CameraController(
      cameras[0],
      ResolutionPreset.medium,
      enableAudio: false,
      imageFormatGroup: Platform.isAndroid
          ? ImageFormatGroup.jpeg
          : ImageFormatGroup.bgra8888,
    );
    try {
      await controller!.initialize();
      controller!.setFlashMode(FlashMode.off);
      setState(() {});

      var scanning = false;
      final stopwatch = Stopwatch();
      stopwatch.start();

      controller!.startImageStream((image) async {
        if (stopwatch.elapsedMilliseconds < 200 || scanning) {
          return;
        }

        PixelFormat format;
        switch (image.format.group) {
          case ImageFormatGroup.bgra8888:
            format = PixelFormat.BGRA8888;
            break;
          case ImageFormatGroup.jpeg:
            format = PixelFormat.JPEG;
            break;
          default:
            controller!.stopImageStream();
            logger.warn(
                "Unsupported image format=${image.format.group} raw=${image.format.raw}");
            setState(() {
              _error = 'Unsupported image format.';
            });
            return;
        }

        try {
          scanning = true;
          final p = image.planes[0];
          final value = await state.native.scanQrCode(
            width: image.width,
            height: image.height,
            format: format,
            buf: p.bytes,
          );
          if (value != null) {
            await widget.linkDevice(value);
            controller!.stopImageStream();
          } else {
            scanning = false;
          }
        } catch (e) {
          logger.warn('Failed to read QR code: $e');
          scanning = false;
        }
      });
    } catch (e) {
      logger.warn('Failed to run camera: $e');
      if (e is CameraException) {
        switch (e.code) {
          case 'CameraAccessDenied':
            setState(() {
              _error = 'Camera access denied.';
            });
            break;
          default:
            // Handle other errors here.
            break;
        }
      } else {
        setState(() {
          _error = 'Unknown error occurred.';
        });
      }
    }
  }

  @override
  void dispose() {
    controller?.dispose();
    super.dispose();
  }

  Widget _content() {
    final theme = Theme.of(context);
    final guide = Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        const Text('1. Open Bolik Timeline on new device'),
        const SizedBox(height: 8),
        const Text('2. Tap "Connect to existing device"'),
        const SizedBox(height: 8),
        const Text('3. Point camera to the code'),
        if (_error != null)
          Padding(
            padding: const EdgeInsets.only(top: 32),
            child: Text(
              _error!,
              style: TextStyle(color: theme.colorScheme.error),
            ),
          )
      ],
    );

    if (controller?.value.isInitialized ?? false) {
      return CameraPreview(
        controller!,
        child: Positioned(
          top: 0,
          left: 0,
          right: 0,
          child: Container(
            color: Colors.white70,
            padding: const EdgeInsets.symmetric(vertical: 8, horizontal: 16),
            child: guide,
          ),
        ),
      );
    }

    return guide;
  }

  @override
  Widget build(BuildContext context) {
    return Center(
      child: ConstrainedBox(
        constraints: const BoxConstraints(maxWidth: 500),
        child: _content(),
      ),
    );
  }
}
