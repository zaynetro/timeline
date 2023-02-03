import 'package:flutter/material.dart';
import 'package:timeline/routes.dart';

class SdkErrorPage extends StatelessWidget {
  const SdkErrorPage({Key? key}) : super(key: key);

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      appBar: AppBar(
        elevation: 0,
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
      body: const Padding(
        padding: EdgeInsets.symmetric(horizontal: 16),
        child: Center(
          child: Text(
            'Fatal error... Sometimes things fail. Please, restart the app.',
            style: TextStyle(fontSize: 18),
          ),
        ),
      ),
    );
  }
}
