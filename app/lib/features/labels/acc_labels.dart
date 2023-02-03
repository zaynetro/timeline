import 'package:flutter/material.dart';
import 'package:provider/provider.dart';
import 'package:timeline/common/models/app_state.dart';
import 'package:timeline/routes.dart';

class AccLabelsPage extends StatefulWidget {
  const AccLabelsPage({super.key});

  @override
  State<StatefulWidget> createState() => _AccLabelsPageState();
}

class _AccLabelsPageState extends State<AccLabelsPage> {
  @override
  Widget build(BuildContext context) {
    final accEdit = context.watch<AccEdit>();
    final accountLabels = accEdit.view.labels;

    return Scaffold(
      appBar: AppBar(
        leading: IconButton(
          onPressed: () {
            BolikRoutes.goBack();
          },
          tooltip: 'Cancel',
          icon: const Icon(Icons.close),
        ),
        title: const Text('Edit labels'),
      ),
      body: ListView(
        children: accountLabels
            .map(
              (l) => ListTile(
                leading: const Icon(Icons.label_outline),
                title: Text(l.name),
                trailing: IconButton(
                  onPressed: () async {
                    accEdit.deleteLabel(labelId: l.id);
                  },
                  icon: const Icon(Icons.delete),
                ),
              ),
            )
            .toList(),
      ),
    );
  }
}
