import 'package:test/test.dart';
import 'package:timeline/bridge_generated.dart';
import 'package:timeline/common/models/dispatcher.dart';

void main() {
  group('App events', () {
    test('Dispatcher', () async {
      final dispatcher = AppEventDispatcher();
      final calls = <String>[];

      dispatcher.addListener<OutputEvent_DocUpdated>((event) =>
          calls.add("OutputEvent_DocUpdated(${event.field0.docId})"));
      dispatcher.addListener<OutputEvent_TimelineUpdated>(
          (event) => calls.add("$event"));
      dispatcher.dispatch(OutputEvent_DocUpdated(DocUpdatedEvent(docId: 'a')));
      dispatcher.dispatch(const OutputEvent_TimelineUpdated());

      expect(
          calls,
          equals([
            "OutputEvent_DocUpdated(a)",
            "OutputEvent.timelineUpdated()",
          ]));
    });

    test('Dispatcher generic', () async {
      final dispatcher = AppEventDispatcher();
      final calls = <String>[];

      dispatcher.addListener<OutputEvent>((event) => calls.add("$event"));
      dispatcher.dispatch(OutputEvent_DocUpdated(DocUpdatedEvent(docId: 'a')));
      dispatcher.dispatch(const OutputEvent_TimelineUpdated());

      expect(
          calls,
          equals([
            "OutputEvent.docUpdated(field0: Instance of 'DocUpdatedEvent')",
            "OutputEvent.timelineUpdated()",
          ]));
    });

    test('Dispatcher remove listener', () async {
      final dispatcher = AppEventDispatcher();
      final calls = <String>[];
      callbackA(event) => calls.add("A: $event");
      callbackB(event) => calls.add("B: $event");

      dispatcher.addListener<OutputEvent>(callbackA);
      dispatcher.addListener<OutputEvent>(callbackB);
      dispatcher.dispatch(const OutputEvent_TimelineUpdated());
      expect(
          calls,
          equals([
            "A: OutputEvent.timelineUpdated()",
            "B: OutputEvent.timelineUpdated()",
          ]));
      calls.clear();

      dispatcher.removeListener(callbackA);
      dispatcher.dispatch(const OutputEvent_TimelineUpdated());
      expect(
          calls,
          equals([
            "B: OutputEvent.timelineUpdated()",
          ]));
    });
  });
}
