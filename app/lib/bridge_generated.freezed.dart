// coverage:ignore-file
// GENERATED CODE - DO NOT MODIFY BY HAND
// ignore_for_file: type=lint
// ignore_for_file: unused_element, deprecated_member_use, deprecated_member_use_from_same_package, use_function_type_syntax_for_parameters, unnecessary_const, avoid_init_to_null, invalid_override_different_default_values_named, prefer_expression_function_bodies, annotate_overrides, invalid_annotation_target, unnecessary_question_mark

part of 'bridge_generated.dart';

// **************************************************************************
// FreezedGenerator
// **************************************************************************

T _$identity<T>(T value) => value;

final _privateConstructorUsedError = UnsupportedError(
    'It seems like you constructed your class using `MyClass._()`. This constructor is only meant to be used by freezed and you are not supposed to need it nor use it.\nPlease check the documentation here for more information: https://github.com/rrousselGit/freezed#custom-getters-and-methods');

/// @nodoc
mixin _$CardChange {
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(CardBlock field0) insert,
    required TResult Function(int position, int len) remove,
    required TResult Function(int position, int len, CardTextAttrs attributes)
        format,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(CardBlock field0)? insert,
    TResult? Function(int position, int len)? remove,
    TResult? Function(int position, int len, CardTextAttrs attributes)? format,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(CardBlock field0)? insert,
    TResult Function(int position, int len)? remove,
    TResult Function(int position, int len, CardTextAttrs attributes)? format,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(CardChange_Insert value) insert,
    required TResult Function(CardChange_Remove value) remove,
    required TResult Function(CardChange_Format value) format,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(CardChange_Insert value)? insert,
    TResult? Function(CardChange_Remove value)? remove,
    TResult? Function(CardChange_Format value)? format,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(CardChange_Insert value)? insert,
    TResult Function(CardChange_Remove value)? remove,
    TResult Function(CardChange_Format value)? format,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $CardChangeCopyWith<$Res> {
  factory $CardChangeCopyWith(
          CardChange value, $Res Function(CardChange) then) =
      _$CardChangeCopyWithImpl<$Res, CardChange>;
}

/// @nodoc
class _$CardChangeCopyWithImpl<$Res, $Val extends CardChange>
    implements $CardChangeCopyWith<$Res> {
  _$CardChangeCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;
}

/// @nodoc
abstract class _$$CardChange_InsertCopyWith<$Res> {
  factory _$$CardChange_InsertCopyWith(
          _$CardChange_Insert value, $Res Function(_$CardChange_Insert) then) =
      __$$CardChange_InsertCopyWithImpl<$Res>;
  @useResult
  $Res call({CardBlock field0});
}

/// @nodoc
class __$$CardChange_InsertCopyWithImpl<$Res>
    extends _$CardChangeCopyWithImpl<$Res, _$CardChange_Insert>
    implements _$$CardChange_InsertCopyWith<$Res> {
  __$$CardChange_InsertCopyWithImpl(
      _$CardChange_Insert _value, $Res Function(_$CardChange_Insert) _then)
      : super(_value, _then);

  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$CardChange_Insert(
      null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as CardBlock,
    ));
  }
}

/// @nodoc

class _$CardChange_Insert implements CardChange_Insert {
  const _$CardChange_Insert(this.field0);

  @override
  final CardBlock field0;

  @override
  String toString() {
    return 'CardChange.insert(field0: $field0)';
  }

  @override
  bool operator ==(dynamic other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$CardChange_Insert &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @JsonKey(ignore: true)
  @override
  @pragma('vm:prefer-inline')
  _$$CardChange_InsertCopyWith<_$CardChange_Insert> get copyWith =>
      __$$CardChange_InsertCopyWithImpl<_$CardChange_Insert>(this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(CardBlock field0) insert,
    required TResult Function(int position, int len) remove,
    required TResult Function(int position, int len, CardTextAttrs attributes)
        format,
  }) {
    return insert(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(CardBlock field0)? insert,
    TResult? Function(int position, int len)? remove,
    TResult? Function(int position, int len, CardTextAttrs attributes)? format,
  }) {
    return insert?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(CardBlock field0)? insert,
    TResult Function(int position, int len)? remove,
    TResult Function(int position, int len, CardTextAttrs attributes)? format,
    required TResult orElse(),
  }) {
    if (insert != null) {
      return insert(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(CardChange_Insert value) insert,
    required TResult Function(CardChange_Remove value) remove,
    required TResult Function(CardChange_Format value) format,
  }) {
    return insert(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(CardChange_Insert value)? insert,
    TResult? Function(CardChange_Remove value)? remove,
    TResult? Function(CardChange_Format value)? format,
  }) {
    return insert?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(CardChange_Insert value)? insert,
    TResult Function(CardChange_Remove value)? remove,
    TResult Function(CardChange_Format value)? format,
    required TResult orElse(),
  }) {
    if (insert != null) {
      return insert(this);
    }
    return orElse();
  }
}

abstract class CardChange_Insert implements CardChange {
  const factory CardChange_Insert(final CardBlock field0) = _$CardChange_Insert;

  CardBlock get field0;
  @JsonKey(ignore: true)
  _$$CardChange_InsertCopyWith<_$CardChange_Insert> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$CardChange_RemoveCopyWith<$Res> {
  factory _$$CardChange_RemoveCopyWith(
          _$CardChange_Remove value, $Res Function(_$CardChange_Remove) then) =
      __$$CardChange_RemoveCopyWithImpl<$Res>;
  @useResult
  $Res call({int position, int len});
}

/// @nodoc
class __$$CardChange_RemoveCopyWithImpl<$Res>
    extends _$CardChangeCopyWithImpl<$Res, _$CardChange_Remove>
    implements _$$CardChange_RemoveCopyWith<$Res> {
  __$$CardChange_RemoveCopyWithImpl(
      _$CardChange_Remove _value, $Res Function(_$CardChange_Remove) _then)
      : super(_value, _then);

  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? position = null,
    Object? len = null,
  }) {
    return _then(_$CardChange_Remove(
      position: null == position
          ? _value.position
          : position // ignore: cast_nullable_to_non_nullable
              as int,
      len: null == len
          ? _value.len
          : len // ignore: cast_nullable_to_non_nullable
              as int,
    ));
  }
}

/// @nodoc

class _$CardChange_Remove implements CardChange_Remove {
  const _$CardChange_Remove({required this.position, required this.len});

  @override
  final int position;
  @override
  final int len;

  @override
  String toString() {
    return 'CardChange.remove(position: $position, len: $len)';
  }

  @override
  bool operator ==(dynamic other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$CardChange_Remove &&
            (identical(other.position, position) ||
                other.position == position) &&
            (identical(other.len, len) || other.len == len));
  }

  @override
  int get hashCode => Object.hash(runtimeType, position, len);

  @JsonKey(ignore: true)
  @override
  @pragma('vm:prefer-inline')
  _$$CardChange_RemoveCopyWith<_$CardChange_Remove> get copyWith =>
      __$$CardChange_RemoveCopyWithImpl<_$CardChange_Remove>(this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(CardBlock field0) insert,
    required TResult Function(int position, int len) remove,
    required TResult Function(int position, int len, CardTextAttrs attributes)
        format,
  }) {
    return remove(position, len);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(CardBlock field0)? insert,
    TResult? Function(int position, int len)? remove,
    TResult? Function(int position, int len, CardTextAttrs attributes)? format,
  }) {
    return remove?.call(position, len);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(CardBlock field0)? insert,
    TResult Function(int position, int len)? remove,
    TResult Function(int position, int len, CardTextAttrs attributes)? format,
    required TResult orElse(),
  }) {
    if (remove != null) {
      return remove(position, len);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(CardChange_Insert value) insert,
    required TResult Function(CardChange_Remove value) remove,
    required TResult Function(CardChange_Format value) format,
  }) {
    return remove(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(CardChange_Insert value)? insert,
    TResult? Function(CardChange_Remove value)? remove,
    TResult? Function(CardChange_Format value)? format,
  }) {
    return remove?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(CardChange_Insert value)? insert,
    TResult Function(CardChange_Remove value)? remove,
    TResult Function(CardChange_Format value)? format,
    required TResult orElse(),
  }) {
    if (remove != null) {
      return remove(this);
    }
    return orElse();
  }
}

abstract class CardChange_Remove implements CardChange {
  const factory CardChange_Remove(
      {required final int position,
      required final int len}) = _$CardChange_Remove;

  int get position;
  int get len;
  @JsonKey(ignore: true)
  _$$CardChange_RemoveCopyWith<_$CardChange_Remove> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$CardChange_FormatCopyWith<$Res> {
  factory _$$CardChange_FormatCopyWith(
          _$CardChange_Format value, $Res Function(_$CardChange_Format) then) =
      __$$CardChange_FormatCopyWithImpl<$Res>;
  @useResult
  $Res call({int position, int len, CardTextAttrs attributes});
}

/// @nodoc
class __$$CardChange_FormatCopyWithImpl<$Res>
    extends _$CardChangeCopyWithImpl<$Res, _$CardChange_Format>
    implements _$$CardChange_FormatCopyWith<$Res> {
  __$$CardChange_FormatCopyWithImpl(
      _$CardChange_Format _value, $Res Function(_$CardChange_Format) _then)
      : super(_value, _then);

  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? position = null,
    Object? len = null,
    Object? attributes = null,
  }) {
    return _then(_$CardChange_Format(
      position: null == position
          ? _value.position
          : position // ignore: cast_nullable_to_non_nullable
              as int,
      len: null == len
          ? _value.len
          : len // ignore: cast_nullable_to_non_nullable
              as int,
      attributes: null == attributes
          ? _value.attributes
          : attributes // ignore: cast_nullable_to_non_nullable
              as CardTextAttrs,
    ));
  }
}

/// @nodoc

class _$CardChange_Format implements CardChange_Format {
  const _$CardChange_Format(
      {required this.position, required this.len, required this.attributes});

  @override
  final int position;
  @override
  final int len;
  @override
  final CardTextAttrs attributes;

  @override
  String toString() {
    return 'CardChange.format(position: $position, len: $len, attributes: $attributes)';
  }

  @override
  bool operator ==(dynamic other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$CardChange_Format &&
            (identical(other.position, position) ||
                other.position == position) &&
            (identical(other.len, len) || other.len == len) &&
            (identical(other.attributes, attributes) ||
                other.attributes == attributes));
  }

  @override
  int get hashCode => Object.hash(runtimeType, position, len, attributes);

  @JsonKey(ignore: true)
  @override
  @pragma('vm:prefer-inline')
  _$$CardChange_FormatCopyWith<_$CardChange_Format> get copyWith =>
      __$$CardChange_FormatCopyWithImpl<_$CardChange_Format>(this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(CardBlock field0) insert,
    required TResult Function(int position, int len) remove,
    required TResult Function(int position, int len, CardTextAttrs attributes)
        format,
  }) {
    return format(position, len, attributes);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(CardBlock field0)? insert,
    TResult? Function(int position, int len)? remove,
    TResult? Function(int position, int len, CardTextAttrs attributes)? format,
  }) {
    return format?.call(position, len, attributes);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(CardBlock field0)? insert,
    TResult Function(int position, int len)? remove,
    TResult Function(int position, int len, CardTextAttrs attributes)? format,
    required TResult orElse(),
  }) {
    if (format != null) {
      return format(position, len, attributes);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(CardChange_Insert value) insert,
    required TResult Function(CardChange_Remove value) remove,
    required TResult Function(CardChange_Format value) format,
  }) {
    return format(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(CardChange_Insert value)? insert,
    TResult? Function(CardChange_Remove value)? remove,
    TResult? Function(CardChange_Format value)? format,
  }) {
    return format?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(CardChange_Insert value)? insert,
    TResult Function(CardChange_Remove value)? remove,
    TResult Function(CardChange_Format value)? format,
    required TResult orElse(),
  }) {
    if (format != null) {
      return format(this);
    }
    return orElse();
  }
}

abstract class CardChange_Format implements CardChange {
  const factory CardChange_Format(
      {required final int position,
      required final int len,
      required final CardTextAttrs attributes}) = _$CardChange_Format;

  int get position;
  int get len;
  CardTextAttrs get attributes;
  @JsonKey(ignore: true)
  _$$CardChange_FormatCopyWith<_$CardChange_Format> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
mixin _$ContentView {
  Object get field0 => throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(CardText field0) text,
    required TResult Function(CardFile field0) file,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(CardText field0)? text,
    TResult? Function(CardFile field0)? file,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(CardText field0)? text,
    TResult Function(CardFile field0)? file,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(ContentView_Text value) text,
    required TResult Function(ContentView_File value) file,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(ContentView_Text value)? text,
    TResult? Function(ContentView_File value)? file,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(ContentView_Text value)? text,
    TResult Function(ContentView_File value)? file,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $ContentViewCopyWith<$Res> {
  factory $ContentViewCopyWith(
          ContentView value, $Res Function(ContentView) then) =
      _$ContentViewCopyWithImpl<$Res, ContentView>;
}

/// @nodoc
class _$ContentViewCopyWithImpl<$Res, $Val extends ContentView>
    implements $ContentViewCopyWith<$Res> {
  _$ContentViewCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;
}

/// @nodoc
abstract class _$$ContentView_TextCopyWith<$Res> {
  factory _$$ContentView_TextCopyWith(
          _$ContentView_Text value, $Res Function(_$ContentView_Text) then) =
      __$$ContentView_TextCopyWithImpl<$Res>;
  @useResult
  $Res call({CardText field0});
}

/// @nodoc
class __$$ContentView_TextCopyWithImpl<$Res>
    extends _$ContentViewCopyWithImpl<$Res, _$ContentView_Text>
    implements _$$ContentView_TextCopyWith<$Res> {
  __$$ContentView_TextCopyWithImpl(
      _$ContentView_Text _value, $Res Function(_$ContentView_Text) _then)
      : super(_value, _then);

  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$ContentView_Text(
      null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as CardText,
    ));
  }
}

/// @nodoc

class _$ContentView_Text implements ContentView_Text {
  const _$ContentView_Text(this.field0);

  @override
  final CardText field0;

  @override
  String toString() {
    return 'ContentView.text(field0: $field0)';
  }

  @override
  bool operator ==(dynamic other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$ContentView_Text &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @JsonKey(ignore: true)
  @override
  @pragma('vm:prefer-inline')
  _$$ContentView_TextCopyWith<_$ContentView_Text> get copyWith =>
      __$$ContentView_TextCopyWithImpl<_$ContentView_Text>(this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(CardText field0) text,
    required TResult Function(CardFile field0) file,
  }) {
    return text(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(CardText field0)? text,
    TResult? Function(CardFile field0)? file,
  }) {
    return text?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(CardText field0)? text,
    TResult Function(CardFile field0)? file,
    required TResult orElse(),
  }) {
    if (text != null) {
      return text(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(ContentView_Text value) text,
    required TResult Function(ContentView_File value) file,
  }) {
    return text(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(ContentView_Text value)? text,
    TResult? Function(ContentView_File value)? file,
  }) {
    return text?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(ContentView_Text value)? text,
    TResult Function(ContentView_File value)? file,
    required TResult orElse(),
  }) {
    if (text != null) {
      return text(this);
    }
    return orElse();
  }
}

abstract class ContentView_Text implements ContentView {
  const factory ContentView_Text(final CardText field0) = _$ContentView_Text;

  @override
  CardText get field0;
  @JsonKey(ignore: true)
  _$$ContentView_TextCopyWith<_$ContentView_Text> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$ContentView_FileCopyWith<$Res> {
  factory _$$ContentView_FileCopyWith(
          _$ContentView_File value, $Res Function(_$ContentView_File) then) =
      __$$ContentView_FileCopyWithImpl<$Res>;
  @useResult
  $Res call({CardFile field0});
}

/// @nodoc
class __$$ContentView_FileCopyWithImpl<$Res>
    extends _$ContentViewCopyWithImpl<$Res, _$ContentView_File>
    implements _$$ContentView_FileCopyWith<$Res> {
  __$$ContentView_FileCopyWithImpl(
      _$ContentView_File _value, $Res Function(_$ContentView_File) _then)
      : super(_value, _then);

  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$ContentView_File(
      null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as CardFile,
    ));
  }
}

/// @nodoc

class _$ContentView_File implements ContentView_File {
  const _$ContentView_File(this.field0);

  @override
  final CardFile field0;

  @override
  String toString() {
    return 'ContentView.file(field0: $field0)';
  }

  @override
  bool operator ==(dynamic other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$ContentView_File &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @JsonKey(ignore: true)
  @override
  @pragma('vm:prefer-inline')
  _$$ContentView_FileCopyWith<_$ContentView_File> get copyWith =>
      __$$ContentView_FileCopyWithImpl<_$ContentView_File>(this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function(CardText field0) text,
    required TResult Function(CardFile field0) file,
  }) {
    return file(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function(CardText field0)? text,
    TResult? Function(CardFile field0)? file,
  }) {
    return file?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function(CardText field0)? text,
    TResult Function(CardFile field0)? file,
    required TResult orElse(),
  }) {
    if (file != null) {
      return file(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(ContentView_Text value) text,
    required TResult Function(ContentView_File value) file,
  }) {
    return file(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(ContentView_Text value)? text,
    TResult? Function(ContentView_File value)? file,
  }) {
    return file?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(ContentView_Text value)? text,
    TResult Function(ContentView_File value)? file,
    required TResult orElse(),
  }) {
    if (file != null) {
      return file(this);
    }
    return orElse();
  }
}

abstract class ContentView_File implements ContentView {
  const factory ContentView_File(final CardFile field0) = _$ContentView_File;

  @override
  CardFile get field0;
  @JsonKey(ignore: true)
  _$$ContentView_FileCopyWith<_$ContentView_File> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
mixin _$OutputEvent {
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() synced,
    required TResult Function() syncFailed,
    required TResult Function() timelineUpdated,
    required TResult Function() preAccount,
    required TResult Function(AccView accView) postAccount,
    required TResult Function(String deviceName) deviceAdded,
    required TResult Function(String docId) docUpdated,
    required TResult Function(String blobId, String path) downloadCompleted,
    required TResult Function(String blobId) downloadFailed,
    required TResult Function(AccView field0) accUpdated,
    required TResult Function(String id) notification,
    required TResult Function() notificationsUpdated,
    required TResult Function() logOut,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? synced,
    TResult? Function()? syncFailed,
    TResult? Function()? timelineUpdated,
    TResult? Function()? preAccount,
    TResult? Function(AccView accView)? postAccount,
    TResult? Function(String deviceName)? deviceAdded,
    TResult? Function(String docId)? docUpdated,
    TResult? Function(String blobId, String path)? downloadCompleted,
    TResult? Function(String blobId)? downloadFailed,
    TResult? Function(AccView field0)? accUpdated,
    TResult? Function(String id)? notification,
    TResult? Function()? notificationsUpdated,
    TResult? Function()? logOut,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? synced,
    TResult Function()? syncFailed,
    TResult Function()? timelineUpdated,
    TResult Function()? preAccount,
    TResult Function(AccView accView)? postAccount,
    TResult Function(String deviceName)? deviceAdded,
    TResult Function(String docId)? docUpdated,
    TResult Function(String blobId, String path)? downloadCompleted,
    TResult Function(String blobId)? downloadFailed,
    TResult Function(AccView field0)? accUpdated,
    TResult Function(String id)? notification,
    TResult Function()? notificationsUpdated,
    TResult Function()? logOut,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(OutputEvent_Synced value) synced,
    required TResult Function(OutputEvent_SyncFailed value) syncFailed,
    required TResult Function(OutputEvent_TimelineUpdated value)
        timelineUpdated,
    required TResult Function(OutputEvent_PreAccount value) preAccount,
    required TResult Function(OutputEvent_PostAccount value) postAccount,
    required TResult Function(OutputEvent_DeviceAdded value) deviceAdded,
    required TResult Function(OutputEvent_DocUpdated value) docUpdated,
    required TResult Function(OutputEvent_DownloadCompleted value)
        downloadCompleted,
    required TResult Function(OutputEvent_DownloadFailed value) downloadFailed,
    required TResult Function(OutputEvent_AccUpdated value) accUpdated,
    required TResult Function(OutputEvent_Notification value) notification,
    required TResult Function(OutputEvent_NotificationsUpdated value)
        notificationsUpdated,
    required TResult Function(OutputEvent_LogOut value) logOut,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(OutputEvent_Synced value)? synced,
    TResult? Function(OutputEvent_SyncFailed value)? syncFailed,
    TResult? Function(OutputEvent_TimelineUpdated value)? timelineUpdated,
    TResult? Function(OutputEvent_PreAccount value)? preAccount,
    TResult? Function(OutputEvent_PostAccount value)? postAccount,
    TResult? Function(OutputEvent_DeviceAdded value)? deviceAdded,
    TResult? Function(OutputEvent_DocUpdated value)? docUpdated,
    TResult? Function(OutputEvent_DownloadCompleted value)? downloadCompleted,
    TResult? Function(OutputEvent_DownloadFailed value)? downloadFailed,
    TResult? Function(OutputEvent_AccUpdated value)? accUpdated,
    TResult? Function(OutputEvent_Notification value)? notification,
    TResult? Function(OutputEvent_NotificationsUpdated value)?
        notificationsUpdated,
    TResult? Function(OutputEvent_LogOut value)? logOut,
  }) =>
      throw _privateConstructorUsedError;
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(OutputEvent_Synced value)? synced,
    TResult Function(OutputEvent_SyncFailed value)? syncFailed,
    TResult Function(OutputEvent_TimelineUpdated value)? timelineUpdated,
    TResult Function(OutputEvent_PreAccount value)? preAccount,
    TResult Function(OutputEvent_PostAccount value)? postAccount,
    TResult Function(OutputEvent_DeviceAdded value)? deviceAdded,
    TResult Function(OutputEvent_DocUpdated value)? docUpdated,
    TResult Function(OutputEvent_DownloadCompleted value)? downloadCompleted,
    TResult Function(OutputEvent_DownloadFailed value)? downloadFailed,
    TResult Function(OutputEvent_AccUpdated value)? accUpdated,
    TResult Function(OutputEvent_Notification value)? notification,
    TResult Function(OutputEvent_NotificationsUpdated value)?
        notificationsUpdated,
    TResult Function(OutputEvent_LogOut value)? logOut,
    required TResult orElse(),
  }) =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class $OutputEventCopyWith<$Res> {
  factory $OutputEventCopyWith(
          OutputEvent value, $Res Function(OutputEvent) then) =
      _$OutputEventCopyWithImpl<$Res, OutputEvent>;
}

/// @nodoc
class _$OutputEventCopyWithImpl<$Res, $Val extends OutputEvent>
    implements $OutputEventCopyWith<$Res> {
  _$OutputEventCopyWithImpl(this._value, this._then);

  // ignore: unused_field
  final $Val _value;
  // ignore: unused_field
  final $Res Function($Val) _then;
}

/// @nodoc
abstract class _$$OutputEvent_SyncedCopyWith<$Res> {
  factory _$$OutputEvent_SyncedCopyWith(_$OutputEvent_Synced value,
          $Res Function(_$OutputEvent_Synced) then) =
      __$$OutputEvent_SyncedCopyWithImpl<$Res>;
}

/// @nodoc
class __$$OutputEvent_SyncedCopyWithImpl<$Res>
    extends _$OutputEventCopyWithImpl<$Res, _$OutputEvent_Synced>
    implements _$$OutputEvent_SyncedCopyWith<$Res> {
  __$$OutputEvent_SyncedCopyWithImpl(
      _$OutputEvent_Synced _value, $Res Function(_$OutputEvent_Synced) _then)
      : super(_value, _then);
}

/// @nodoc

class _$OutputEvent_Synced implements OutputEvent_Synced {
  const _$OutputEvent_Synced();

  @override
  String toString() {
    return 'OutputEvent.synced()';
  }

  @override
  bool operator ==(dynamic other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType && other is _$OutputEvent_Synced);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() synced,
    required TResult Function() syncFailed,
    required TResult Function() timelineUpdated,
    required TResult Function() preAccount,
    required TResult Function(AccView accView) postAccount,
    required TResult Function(String deviceName) deviceAdded,
    required TResult Function(String docId) docUpdated,
    required TResult Function(String blobId, String path) downloadCompleted,
    required TResult Function(String blobId) downloadFailed,
    required TResult Function(AccView field0) accUpdated,
    required TResult Function(String id) notification,
    required TResult Function() notificationsUpdated,
    required TResult Function() logOut,
  }) {
    return synced();
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? synced,
    TResult? Function()? syncFailed,
    TResult? Function()? timelineUpdated,
    TResult? Function()? preAccount,
    TResult? Function(AccView accView)? postAccount,
    TResult? Function(String deviceName)? deviceAdded,
    TResult? Function(String docId)? docUpdated,
    TResult? Function(String blobId, String path)? downloadCompleted,
    TResult? Function(String blobId)? downloadFailed,
    TResult? Function(AccView field0)? accUpdated,
    TResult? Function(String id)? notification,
    TResult? Function()? notificationsUpdated,
    TResult? Function()? logOut,
  }) {
    return synced?.call();
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? synced,
    TResult Function()? syncFailed,
    TResult Function()? timelineUpdated,
    TResult Function()? preAccount,
    TResult Function(AccView accView)? postAccount,
    TResult Function(String deviceName)? deviceAdded,
    TResult Function(String docId)? docUpdated,
    TResult Function(String blobId, String path)? downloadCompleted,
    TResult Function(String blobId)? downloadFailed,
    TResult Function(AccView field0)? accUpdated,
    TResult Function(String id)? notification,
    TResult Function()? notificationsUpdated,
    TResult Function()? logOut,
    required TResult orElse(),
  }) {
    if (synced != null) {
      return synced();
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(OutputEvent_Synced value) synced,
    required TResult Function(OutputEvent_SyncFailed value) syncFailed,
    required TResult Function(OutputEvent_TimelineUpdated value)
        timelineUpdated,
    required TResult Function(OutputEvent_PreAccount value) preAccount,
    required TResult Function(OutputEvent_PostAccount value) postAccount,
    required TResult Function(OutputEvent_DeviceAdded value) deviceAdded,
    required TResult Function(OutputEvent_DocUpdated value) docUpdated,
    required TResult Function(OutputEvent_DownloadCompleted value)
        downloadCompleted,
    required TResult Function(OutputEvent_DownloadFailed value) downloadFailed,
    required TResult Function(OutputEvent_AccUpdated value) accUpdated,
    required TResult Function(OutputEvent_Notification value) notification,
    required TResult Function(OutputEvent_NotificationsUpdated value)
        notificationsUpdated,
    required TResult Function(OutputEvent_LogOut value) logOut,
  }) {
    return synced(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(OutputEvent_Synced value)? synced,
    TResult? Function(OutputEvent_SyncFailed value)? syncFailed,
    TResult? Function(OutputEvent_TimelineUpdated value)? timelineUpdated,
    TResult? Function(OutputEvent_PreAccount value)? preAccount,
    TResult? Function(OutputEvent_PostAccount value)? postAccount,
    TResult? Function(OutputEvent_DeviceAdded value)? deviceAdded,
    TResult? Function(OutputEvent_DocUpdated value)? docUpdated,
    TResult? Function(OutputEvent_DownloadCompleted value)? downloadCompleted,
    TResult? Function(OutputEvent_DownloadFailed value)? downloadFailed,
    TResult? Function(OutputEvent_AccUpdated value)? accUpdated,
    TResult? Function(OutputEvent_Notification value)? notification,
    TResult? Function(OutputEvent_NotificationsUpdated value)?
        notificationsUpdated,
    TResult? Function(OutputEvent_LogOut value)? logOut,
  }) {
    return synced?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(OutputEvent_Synced value)? synced,
    TResult Function(OutputEvent_SyncFailed value)? syncFailed,
    TResult Function(OutputEvent_TimelineUpdated value)? timelineUpdated,
    TResult Function(OutputEvent_PreAccount value)? preAccount,
    TResult Function(OutputEvent_PostAccount value)? postAccount,
    TResult Function(OutputEvent_DeviceAdded value)? deviceAdded,
    TResult Function(OutputEvent_DocUpdated value)? docUpdated,
    TResult Function(OutputEvent_DownloadCompleted value)? downloadCompleted,
    TResult Function(OutputEvent_DownloadFailed value)? downloadFailed,
    TResult Function(OutputEvent_AccUpdated value)? accUpdated,
    TResult Function(OutputEvent_Notification value)? notification,
    TResult Function(OutputEvent_NotificationsUpdated value)?
        notificationsUpdated,
    TResult Function(OutputEvent_LogOut value)? logOut,
    required TResult orElse(),
  }) {
    if (synced != null) {
      return synced(this);
    }
    return orElse();
  }
}

abstract class OutputEvent_Synced implements OutputEvent {
  const factory OutputEvent_Synced() = _$OutputEvent_Synced;
}

/// @nodoc
abstract class _$$OutputEvent_SyncFailedCopyWith<$Res> {
  factory _$$OutputEvent_SyncFailedCopyWith(_$OutputEvent_SyncFailed value,
          $Res Function(_$OutputEvent_SyncFailed) then) =
      __$$OutputEvent_SyncFailedCopyWithImpl<$Res>;
}

/// @nodoc
class __$$OutputEvent_SyncFailedCopyWithImpl<$Res>
    extends _$OutputEventCopyWithImpl<$Res, _$OutputEvent_SyncFailed>
    implements _$$OutputEvent_SyncFailedCopyWith<$Res> {
  __$$OutputEvent_SyncFailedCopyWithImpl(_$OutputEvent_SyncFailed _value,
      $Res Function(_$OutputEvent_SyncFailed) _then)
      : super(_value, _then);
}

/// @nodoc

class _$OutputEvent_SyncFailed implements OutputEvent_SyncFailed {
  const _$OutputEvent_SyncFailed();

  @override
  String toString() {
    return 'OutputEvent.syncFailed()';
  }

  @override
  bool operator ==(dynamic other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType && other is _$OutputEvent_SyncFailed);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() synced,
    required TResult Function() syncFailed,
    required TResult Function() timelineUpdated,
    required TResult Function() preAccount,
    required TResult Function(AccView accView) postAccount,
    required TResult Function(String deviceName) deviceAdded,
    required TResult Function(String docId) docUpdated,
    required TResult Function(String blobId, String path) downloadCompleted,
    required TResult Function(String blobId) downloadFailed,
    required TResult Function(AccView field0) accUpdated,
    required TResult Function(String id) notification,
    required TResult Function() notificationsUpdated,
    required TResult Function() logOut,
  }) {
    return syncFailed();
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? synced,
    TResult? Function()? syncFailed,
    TResult? Function()? timelineUpdated,
    TResult? Function()? preAccount,
    TResult? Function(AccView accView)? postAccount,
    TResult? Function(String deviceName)? deviceAdded,
    TResult? Function(String docId)? docUpdated,
    TResult? Function(String blobId, String path)? downloadCompleted,
    TResult? Function(String blobId)? downloadFailed,
    TResult? Function(AccView field0)? accUpdated,
    TResult? Function(String id)? notification,
    TResult? Function()? notificationsUpdated,
    TResult? Function()? logOut,
  }) {
    return syncFailed?.call();
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? synced,
    TResult Function()? syncFailed,
    TResult Function()? timelineUpdated,
    TResult Function()? preAccount,
    TResult Function(AccView accView)? postAccount,
    TResult Function(String deviceName)? deviceAdded,
    TResult Function(String docId)? docUpdated,
    TResult Function(String blobId, String path)? downloadCompleted,
    TResult Function(String blobId)? downloadFailed,
    TResult Function(AccView field0)? accUpdated,
    TResult Function(String id)? notification,
    TResult Function()? notificationsUpdated,
    TResult Function()? logOut,
    required TResult orElse(),
  }) {
    if (syncFailed != null) {
      return syncFailed();
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(OutputEvent_Synced value) synced,
    required TResult Function(OutputEvent_SyncFailed value) syncFailed,
    required TResult Function(OutputEvent_TimelineUpdated value)
        timelineUpdated,
    required TResult Function(OutputEvent_PreAccount value) preAccount,
    required TResult Function(OutputEvent_PostAccount value) postAccount,
    required TResult Function(OutputEvent_DeviceAdded value) deviceAdded,
    required TResult Function(OutputEvent_DocUpdated value) docUpdated,
    required TResult Function(OutputEvent_DownloadCompleted value)
        downloadCompleted,
    required TResult Function(OutputEvent_DownloadFailed value) downloadFailed,
    required TResult Function(OutputEvent_AccUpdated value) accUpdated,
    required TResult Function(OutputEvent_Notification value) notification,
    required TResult Function(OutputEvent_NotificationsUpdated value)
        notificationsUpdated,
    required TResult Function(OutputEvent_LogOut value) logOut,
  }) {
    return syncFailed(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(OutputEvent_Synced value)? synced,
    TResult? Function(OutputEvent_SyncFailed value)? syncFailed,
    TResult? Function(OutputEvent_TimelineUpdated value)? timelineUpdated,
    TResult? Function(OutputEvent_PreAccount value)? preAccount,
    TResult? Function(OutputEvent_PostAccount value)? postAccount,
    TResult? Function(OutputEvent_DeviceAdded value)? deviceAdded,
    TResult? Function(OutputEvent_DocUpdated value)? docUpdated,
    TResult? Function(OutputEvent_DownloadCompleted value)? downloadCompleted,
    TResult? Function(OutputEvent_DownloadFailed value)? downloadFailed,
    TResult? Function(OutputEvent_AccUpdated value)? accUpdated,
    TResult? Function(OutputEvent_Notification value)? notification,
    TResult? Function(OutputEvent_NotificationsUpdated value)?
        notificationsUpdated,
    TResult? Function(OutputEvent_LogOut value)? logOut,
  }) {
    return syncFailed?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(OutputEvent_Synced value)? synced,
    TResult Function(OutputEvent_SyncFailed value)? syncFailed,
    TResult Function(OutputEvent_TimelineUpdated value)? timelineUpdated,
    TResult Function(OutputEvent_PreAccount value)? preAccount,
    TResult Function(OutputEvent_PostAccount value)? postAccount,
    TResult Function(OutputEvent_DeviceAdded value)? deviceAdded,
    TResult Function(OutputEvent_DocUpdated value)? docUpdated,
    TResult Function(OutputEvent_DownloadCompleted value)? downloadCompleted,
    TResult Function(OutputEvent_DownloadFailed value)? downloadFailed,
    TResult Function(OutputEvent_AccUpdated value)? accUpdated,
    TResult Function(OutputEvent_Notification value)? notification,
    TResult Function(OutputEvent_NotificationsUpdated value)?
        notificationsUpdated,
    TResult Function(OutputEvent_LogOut value)? logOut,
    required TResult orElse(),
  }) {
    if (syncFailed != null) {
      return syncFailed(this);
    }
    return orElse();
  }
}

abstract class OutputEvent_SyncFailed implements OutputEvent {
  const factory OutputEvent_SyncFailed() = _$OutputEvent_SyncFailed;
}

/// @nodoc
abstract class _$$OutputEvent_TimelineUpdatedCopyWith<$Res> {
  factory _$$OutputEvent_TimelineUpdatedCopyWith(
          _$OutputEvent_TimelineUpdated value,
          $Res Function(_$OutputEvent_TimelineUpdated) then) =
      __$$OutputEvent_TimelineUpdatedCopyWithImpl<$Res>;
}

/// @nodoc
class __$$OutputEvent_TimelineUpdatedCopyWithImpl<$Res>
    extends _$OutputEventCopyWithImpl<$Res, _$OutputEvent_TimelineUpdated>
    implements _$$OutputEvent_TimelineUpdatedCopyWith<$Res> {
  __$$OutputEvent_TimelineUpdatedCopyWithImpl(
      _$OutputEvent_TimelineUpdated _value,
      $Res Function(_$OutputEvent_TimelineUpdated) _then)
      : super(_value, _then);
}

/// @nodoc

class _$OutputEvent_TimelineUpdated implements OutputEvent_TimelineUpdated {
  const _$OutputEvent_TimelineUpdated();

  @override
  String toString() {
    return 'OutputEvent.timelineUpdated()';
  }

  @override
  bool operator ==(dynamic other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$OutputEvent_TimelineUpdated);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() synced,
    required TResult Function() syncFailed,
    required TResult Function() timelineUpdated,
    required TResult Function() preAccount,
    required TResult Function(AccView accView) postAccount,
    required TResult Function(String deviceName) deviceAdded,
    required TResult Function(String docId) docUpdated,
    required TResult Function(String blobId, String path) downloadCompleted,
    required TResult Function(String blobId) downloadFailed,
    required TResult Function(AccView field0) accUpdated,
    required TResult Function(String id) notification,
    required TResult Function() notificationsUpdated,
    required TResult Function() logOut,
  }) {
    return timelineUpdated();
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? synced,
    TResult? Function()? syncFailed,
    TResult? Function()? timelineUpdated,
    TResult? Function()? preAccount,
    TResult? Function(AccView accView)? postAccount,
    TResult? Function(String deviceName)? deviceAdded,
    TResult? Function(String docId)? docUpdated,
    TResult? Function(String blobId, String path)? downloadCompleted,
    TResult? Function(String blobId)? downloadFailed,
    TResult? Function(AccView field0)? accUpdated,
    TResult? Function(String id)? notification,
    TResult? Function()? notificationsUpdated,
    TResult? Function()? logOut,
  }) {
    return timelineUpdated?.call();
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? synced,
    TResult Function()? syncFailed,
    TResult Function()? timelineUpdated,
    TResult Function()? preAccount,
    TResult Function(AccView accView)? postAccount,
    TResult Function(String deviceName)? deviceAdded,
    TResult Function(String docId)? docUpdated,
    TResult Function(String blobId, String path)? downloadCompleted,
    TResult Function(String blobId)? downloadFailed,
    TResult Function(AccView field0)? accUpdated,
    TResult Function(String id)? notification,
    TResult Function()? notificationsUpdated,
    TResult Function()? logOut,
    required TResult orElse(),
  }) {
    if (timelineUpdated != null) {
      return timelineUpdated();
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(OutputEvent_Synced value) synced,
    required TResult Function(OutputEvent_SyncFailed value) syncFailed,
    required TResult Function(OutputEvent_TimelineUpdated value)
        timelineUpdated,
    required TResult Function(OutputEvent_PreAccount value) preAccount,
    required TResult Function(OutputEvent_PostAccount value) postAccount,
    required TResult Function(OutputEvent_DeviceAdded value) deviceAdded,
    required TResult Function(OutputEvent_DocUpdated value) docUpdated,
    required TResult Function(OutputEvent_DownloadCompleted value)
        downloadCompleted,
    required TResult Function(OutputEvent_DownloadFailed value) downloadFailed,
    required TResult Function(OutputEvent_AccUpdated value) accUpdated,
    required TResult Function(OutputEvent_Notification value) notification,
    required TResult Function(OutputEvent_NotificationsUpdated value)
        notificationsUpdated,
    required TResult Function(OutputEvent_LogOut value) logOut,
  }) {
    return timelineUpdated(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(OutputEvent_Synced value)? synced,
    TResult? Function(OutputEvent_SyncFailed value)? syncFailed,
    TResult? Function(OutputEvent_TimelineUpdated value)? timelineUpdated,
    TResult? Function(OutputEvent_PreAccount value)? preAccount,
    TResult? Function(OutputEvent_PostAccount value)? postAccount,
    TResult? Function(OutputEvent_DeviceAdded value)? deviceAdded,
    TResult? Function(OutputEvent_DocUpdated value)? docUpdated,
    TResult? Function(OutputEvent_DownloadCompleted value)? downloadCompleted,
    TResult? Function(OutputEvent_DownloadFailed value)? downloadFailed,
    TResult? Function(OutputEvent_AccUpdated value)? accUpdated,
    TResult? Function(OutputEvent_Notification value)? notification,
    TResult? Function(OutputEvent_NotificationsUpdated value)?
        notificationsUpdated,
    TResult? Function(OutputEvent_LogOut value)? logOut,
  }) {
    return timelineUpdated?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(OutputEvent_Synced value)? synced,
    TResult Function(OutputEvent_SyncFailed value)? syncFailed,
    TResult Function(OutputEvent_TimelineUpdated value)? timelineUpdated,
    TResult Function(OutputEvent_PreAccount value)? preAccount,
    TResult Function(OutputEvent_PostAccount value)? postAccount,
    TResult Function(OutputEvent_DeviceAdded value)? deviceAdded,
    TResult Function(OutputEvent_DocUpdated value)? docUpdated,
    TResult Function(OutputEvent_DownloadCompleted value)? downloadCompleted,
    TResult Function(OutputEvent_DownloadFailed value)? downloadFailed,
    TResult Function(OutputEvent_AccUpdated value)? accUpdated,
    TResult Function(OutputEvent_Notification value)? notification,
    TResult Function(OutputEvent_NotificationsUpdated value)?
        notificationsUpdated,
    TResult Function(OutputEvent_LogOut value)? logOut,
    required TResult orElse(),
  }) {
    if (timelineUpdated != null) {
      return timelineUpdated(this);
    }
    return orElse();
  }
}

abstract class OutputEvent_TimelineUpdated implements OutputEvent {
  const factory OutputEvent_TimelineUpdated() = _$OutputEvent_TimelineUpdated;
}

/// @nodoc
abstract class _$$OutputEvent_PreAccountCopyWith<$Res> {
  factory _$$OutputEvent_PreAccountCopyWith(_$OutputEvent_PreAccount value,
          $Res Function(_$OutputEvent_PreAccount) then) =
      __$$OutputEvent_PreAccountCopyWithImpl<$Res>;
}

/// @nodoc
class __$$OutputEvent_PreAccountCopyWithImpl<$Res>
    extends _$OutputEventCopyWithImpl<$Res, _$OutputEvent_PreAccount>
    implements _$$OutputEvent_PreAccountCopyWith<$Res> {
  __$$OutputEvent_PreAccountCopyWithImpl(_$OutputEvent_PreAccount _value,
      $Res Function(_$OutputEvent_PreAccount) _then)
      : super(_value, _then);
}

/// @nodoc

class _$OutputEvent_PreAccount implements OutputEvent_PreAccount {
  const _$OutputEvent_PreAccount();

  @override
  String toString() {
    return 'OutputEvent.preAccount()';
  }

  @override
  bool operator ==(dynamic other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType && other is _$OutputEvent_PreAccount);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() synced,
    required TResult Function() syncFailed,
    required TResult Function() timelineUpdated,
    required TResult Function() preAccount,
    required TResult Function(AccView accView) postAccount,
    required TResult Function(String deviceName) deviceAdded,
    required TResult Function(String docId) docUpdated,
    required TResult Function(String blobId, String path) downloadCompleted,
    required TResult Function(String blobId) downloadFailed,
    required TResult Function(AccView field0) accUpdated,
    required TResult Function(String id) notification,
    required TResult Function() notificationsUpdated,
    required TResult Function() logOut,
  }) {
    return preAccount();
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? synced,
    TResult? Function()? syncFailed,
    TResult? Function()? timelineUpdated,
    TResult? Function()? preAccount,
    TResult? Function(AccView accView)? postAccount,
    TResult? Function(String deviceName)? deviceAdded,
    TResult? Function(String docId)? docUpdated,
    TResult? Function(String blobId, String path)? downloadCompleted,
    TResult? Function(String blobId)? downloadFailed,
    TResult? Function(AccView field0)? accUpdated,
    TResult? Function(String id)? notification,
    TResult? Function()? notificationsUpdated,
    TResult? Function()? logOut,
  }) {
    return preAccount?.call();
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? synced,
    TResult Function()? syncFailed,
    TResult Function()? timelineUpdated,
    TResult Function()? preAccount,
    TResult Function(AccView accView)? postAccount,
    TResult Function(String deviceName)? deviceAdded,
    TResult Function(String docId)? docUpdated,
    TResult Function(String blobId, String path)? downloadCompleted,
    TResult Function(String blobId)? downloadFailed,
    TResult Function(AccView field0)? accUpdated,
    TResult Function(String id)? notification,
    TResult Function()? notificationsUpdated,
    TResult Function()? logOut,
    required TResult orElse(),
  }) {
    if (preAccount != null) {
      return preAccount();
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(OutputEvent_Synced value) synced,
    required TResult Function(OutputEvent_SyncFailed value) syncFailed,
    required TResult Function(OutputEvent_TimelineUpdated value)
        timelineUpdated,
    required TResult Function(OutputEvent_PreAccount value) preAccount,
    required TResult Function(OutputEvent_PostAccount value) postAccount,
    required TResult Function(OutputEvent_DeviceAdded value) deviceAdded,
    required TResult Function(OutputEvent_DocUpdated value) docUpdated,
    required TResult Function(OutputEvent_DownloadCompleted value)
        downloadCompleted,
    required TResult Function(OutputEvent_DownloadFailed value) downloadFailed,
    required TResult Function(OutputEvent_AccUpdated value) accUpdated,
    required TResult Function(OutputEvent_Notification value) notification,
    required TResult Function(OutputEvent_NotificationsUpdated value)
        notificationsUpdated,
    required TResult Function(OutputEvent_LogOut value) logOut,
  }) {
    return preAccount(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(OutputEvent_Synced value)? synced,
    TResult? Function(OutputEvent_SyncFailed value)? syncFailed,
    TResult? Function(OutputEvent_TimelineUpdated value)? timelineUpdated,
    TResult? Function(OutputEvent_PreAccount value)? preAccount,
    TResult? Function(OutputEvent_PostAccount value)? postAccount,
    TResult? Function(OutputEvent_DeviceAdded value)? deviceAdded,
    TResult? Function(OutputEvent_DocUpdated value)? docUpdated,
    TResult? Function(OutputEvent_DownloadCompleted value)? downloadCompleted,
    TResult? Function(OutputEvent_DownloadFailed value)? downloadFailed,
    TResult? Function(OutputEvent_AccUpdated value)? accUpdated,
    TResult? Function(OutputEvent_Notification value)? notification,
    TResult? Function(OutputEvent_NotificationsUpdated value)?
        notificationsUpdated,
    TResult? Function(OutputEvent_LogOut value)? logOut,
  }) {
    return preAccount?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(OutputEvent_Synced value)? synced,
    TResult Function(OutputEvent_SyncFailed value)? syncFailed,
    TResult Function(OutputEvent_TimelineUpdated value)? timelineUpdated,
    TResult Function(OutputEvent_PreAccount value)? preAccount,
    TResult Function(OutputEvent_PostAccount value)? postAccount,
    TResult Function(OutputEvent_DeviceAdded value)? deviceAdded,
    TResult Function(OutputEvent_DocUpdated value)? docUpdated,
    TResult Function(OutputEvent_DownloadCompleted value)? downloadCompleted,
    TResult Function(OutputEvent_DownloadFailed value)? downloadFailed,
    TResult Function(OutputEvent_AccUpdated value)? accUpdated,
    TResult Function(OutputEvent_Notification value)? notification,
    TResult Function(OutputEvent_NotificationsUpdated value)?
        notificationsUpdated,
    TResult Function(OutputEvent_LogOut value)? logOut,
    required TResult orElse(),
  }) {
    if (preAccount != null) {
      return preAccount(this);
    }
    return orElse();
  }
}

abstract class OutputEvent_PreAccount implements OutputEvent {
  const factory OutputEvent_PreAccount() = _$OutputEvent_PreAccount;
}

/// @nodoc
abstract class _$$OutputEvent_PostAccountCopyWith<$Res> {
  factory _$$OutputEvent_PostAccountCopyWith(_$OutputEvent_PostAccount value,
          $Res Function(_$OutputEvent_PostAccount) then) =
      __$$OutputEvent_PostAccountCopyWithImpl<$Res>;
  @useResult
  $Res call({AccView accView});
}

/// @nodoc
class __$$OutputEvent_PostAccountCopyWithImpl<$Res>
    extends _$OutputEventCopyWithImpl<$Res, _$OutputEvent_PostAccount>
    implements _$$OutputEvent_PostAccountCopyWith<$Res> {
  __$$OutputEvent_PostAccountCopyWithImpl(_$OutputEvent_PostAccount _value,
      $Res Function(_$OutputEvent_PostAccount) _then)
      : super(_value, _then);

  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? accView = null,
  }) {
    return _then(_$OutputEvent_PostAccount(
      accView: null == accView
          ? _value.accView
          : accView // ignore: cast_nullable_to_non_nullable
              as AccView,
    ));
  }
}

/// @nodoc

class _$OutputEvent_PostAccount implements OutputEvent_PostAccount {
  const _$OutputEvent_PostAccount({required this.accView});

  @override
  final AccView accView;

  @override
  String toString() {
    return 'OutputEvent.postAccount(accView: $accView)';
  }

  @override
  bool operator ==(dynamic other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$OutputEvent_PostAccount &&
            (identical(other.accView, accView) || other.accView == accView));
  }

  @override
  int get hashCode => Object.hash(runtimeType, accView);

  @JsonKey(ignore: true)
  @override
  @pragma('vm:prefer-inline')
  _$$OutputEvent_PostAccountCopyWith<_$OutputEvent_PostAccount> get copyWith =>
      __$$OutputEvent_PostAccountCopyWithImpl<_$OutputEvent_PostAccount>(
          this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() synced,
    required TResult Function() syncFailed,
    required TResult Function() timelineUpdated,
    required TResult Function() preAccount,
    required TResult Function(AccView accView) postAccount,
    required TResult Function(String deviceName) deviceAdded,
    required TResult Function(String docId) docUpdated,
    required TResult Function(String blobId, String path) downloadCompleted,
    required TResult Function(String blobId) downloadFailed,
    required TResult Function(AccView field0) accUpdated,
    required TResult Function(String id) notification,
    required TResult Function() notificationsUpdated,
    required TResult Function() logOut,
  }) {
    return postAccount(accView);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? synced,
    TResult? Function()? syncFailed,
    TResult? Function()? timelineUpdated,
    TResult? Function()? preAccount,
    TResult? Function(AccView accView)? postAccount,
    TResult? Function(String deviceName)? deviceAdded,
    TResult? Function(String docId)? docUpdated,
    TResult? Function(String blobId, String path)? downloadCompleted,
    TResult? Function(String blobId)? downloadFailed,
    TResult? Function(AccView field0)? accUpdated,
    TResult? Function(String id)? notification,
    TResult? Function()? notificationsUpdated,
    TResult? Function()? logOut,
  }) {
    return postAccount?.call(accView);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? synced,
    TResult Function()? syncFailed,
    TResult Function()? timelineUpdated,
    TResult Function()? preAccount,
    TResult Function(AccView accView)? postAccount,
    TResult Function(String deviceName)? deviceAdded,
    TResult Function(String docId)? docUpdated,
    TResult Function(String blobId, String path)? downloadCompleted,
    TResult Function(String blobId)? downloadFailed,
    TResult Function(AccView field0)? accUpdated,
    TResult Function(String id)? notification,
    TResult Function()? notificationsUpdated,
    TResult Function()? logOut,
    required TResult orElse(),
  }) {
    if (postAccount != null) {
      return postAccount(accView);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(OutputEvent_Synced value) synced,
    required TResult Function(OutputEvent_SyncFailed value) syncFailed,
    required TResult Function(OutputEvent_TimelineUpdated value)
        timelineUpdated,
    required TResult Function(OutputEvent_PreAccount value) preAccount,
    required TResult Function(OutputEvent_PostAccount value) postAccount,
    required TResult Function(OutputEvent_DeviceAdded value) deviceAdded,
    required TResult Function(OutputEvent_DocUpdated value) docUpdated,
    required TResult Function(OutputEvent_DownloadCompleted value)
        downloadCompleted,
    required TResult Function(OutputEvent_DownloadFailed value) downloadFailed,
    required TResult Function(OutputEvent_AccUpdated value) accUpdated,
    required TResult Function(OutputEvent_Notification value) notification,
    required TResult Function(OutputEvent_NotificationsUpdated value)
        notificationsUpdated,
    required TResult Function(OutputEvent_LogOut value) logOut,
  }) {
    return postAccount(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(OutputEvent_Synced value)? synced,
    TResult? Function(OutputEvent_SyncFailed value)? syncFailed,
    TResult? Function(OutputEvent_TimelineUpdated value)? timelineUpdated,
    TResult? Function(OutputEvent_PreAccount value)? preAccount,
    TResult? Function(OutputEvent_PostAccount value)? postAccount,
    TResult? Function(OutputEvent_DeviceAdded value)? deviceAdded,
    TResult? Function(OutputEvent_DocUpdated value)? docUpdated,
    TResult? Function(OutputEvent_DownloadCompleted value)? downloadCompleted,
    TResult? Function(OutputEvent_DownloadFailed value)? downloadFailed,
    TResult? Function(OutputEvent_AccUpdated value)? accUpdated,
    TResult? Function(OutputEvent_Notification value)? notification,
    TResult? Function(OutputEvent_NotificationsUpdated value)?
        notificationsUpdated,
    TResult? Function(OutputEvent_LogOut value)? logOut,
  }) {
    return postAccount?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(OutputEvent_Synced value)? synced,
    TResult Function(OutputEvent_SyncFailed value)? syncFailed,
    TResult Function(OutputEvent_TimelineUpdated value)? timelineUpdated,
    TResult Function(OutputEvent_PreAccount value)? preAccount,
    TResult Function(OutputEvent_PostAccount value)? postAccount,
    TResult Function(OutputEvent_DeviceAdded value)? deviceAdded,
    TResult Function(OutputEvent_DocUpdated value)? docUpdated,
    TResult Function(OutputEvent_DownloadCompleted value)? downloadCompleted,
    TResult Function(OutputEvent_DownloadFailed value)? downloadFailed,
    TResult Function(OutputEvent_AccUpdated value)? accUpdated,
    TResult Function(OutputEvent_Notification value)? notification,
    TResult Function(OutputEvent_NotificationsUpdated value)?
        notificationsUpdated,
    TResult Function(OutputEvent_LogOut value)? logOut,
    required TResult orElse(),
  }) {
    if (postAccount != null) {
      return postAccount(this);
    }
    return orElse();
  }
}

abstract class OutputEvent_PostAccount implements OutputEvent {
  const factory OutputEvent_PostAccount({required final AccView accView}) =
      _$OutputEvent_PostAccount;

  AccView get accView;
  @JsonKey(ignore: true)
  _$$OutputEvent_PostAccountCopyWith<_$OutputEvent_PostAccount> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$OutputEvent_DeviceAddedCopyWith<$Res> {
  factory _$$OutputEvent_DeviceAddedCopyWith(_$OutputEvent_DeviceAdded value,
          $Res Function(_$OutputEvent_DeviceAdded) then) =
      __$$OutputEvent_DeviceAddedCopyWithImpl<$Res>;
  @useResult
  $Res call({String deviceName});
}

/// @nodoc
class __$$OutputEvent_DeviceAddedCopyWithImpl<$Res>
    extends _$OutputEventCopyWithImpl<$Res, _$OutputEvent_DeviceAdded>
    implements _$$OutputEvent_DeviceAddedCopyWith<$Res> {
  __$$OutputEvent_DeviceAddedCopyWithImpl(_$OutputEvent_DeviceAdded _value,
      $Res Function(_$OutputEvent_DeviceAdded) _then)
      : super(_value, _then);

  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? deviceName = null,
  }) {
    return _then(_$OutputEvent_DeviceAdded(
      deviceName: null == deviceName
          ? _value.deviceName
          : deviceName // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class _$OutputEvent_DeviceAdded implements OutputEvent_DeviceAdded {
  const _$OutputEvent_DeviceAdded({required this.deviceName});

  @override
  final String deviceName;

  @override
  String toString() {
    return 'OutputEvent.deviceAdded(deviceName: $deviceName)';
  }

  @override
  bool operator ==(dynamic other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$OutputEvent_DeviceAdded &&
            (identical(other.deviceName, deviceName) ||
                other.deviceName == deviceName));
  }

  @override
  int get hashCode => Object.hash(runtimeType, deviceName);

  @JsonKey(ignore: true)
  @override
  @pragma('vm:prefer-inline')
  _$$OutputEvent_DeviceAddedCopyWith<_$OutputEvent_DeviceAdded> get copyWith =>
      __$$OutputEvent_DeviceAddedCopyWithImpl<_$OutputEvent_DeviceAdded>(
          this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() synced,
    required TResult Function() syncFailed,
    required TResult Function() timelineUpdated,
    required TResult Function() preAccount,
    required TResult Function(AccView accView) postAccount,
    required TResult Function(String deviceName) deviceAdded,
    required TResult Function(String docId) docUpdated,
    required TResult Function(String blobId, String path) downloadCompleted,
    required TResult Function(String blobId) downloadFailed,
    required TResult Function(AccView field0) accUpdated,
    required TResult Function(String id) notification,
    required TResult Function() notificationsUpdated,
    required TResult Function() logOut,
  }) {
    return deviceAdded(deviceName);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? synced,
    TResult? Function()? syncFailed,
    TResult? Function()? timelineUpdated,
    TResult? Function()? preAccount,
    TResult? Function(AccView accView)? postAccount,
    TResult? Function(String deviceName)? deviceAdded,
    TResult? Function(String docId)? docUpdated,
    TResult? Function(String blobId, String path)? downloadCompleted,
    TResult? Function(String blobId)? downloadFailed,
    TResult? Function(AccView field0)? accUpdated,
    TResult? Function(String id)? notification,
    TResult? Function()? notificationsUpdated,
    TResult? Function()? logOut,
  }) {
    return deviceAdded?.call(deviceName);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? synced,
    TResult Function()? syncFailed,
    TResult Function()? timelineUpdated,
    TResult Function()? preAccount,
    TResult Function(AccView accView)? postAccount,
    TResult Function(String deviceName)? deviceAdded,
    TResult Function(String docId)? docUpdated,
    TResult Function(String blobId, String path)? downloadCompleted,
    TResult Function(String blobId)? downloadFailed,
    TResult Function(AccView field0)? accUpdated,
    TResult Function(String id)? notification,
    TResult Function()? notificationsUpdated,
    TResult Function()? logOut,
    required TResult orElse(),
  }) {
    if (deviceAdded != null) {
      return deviceAdded(deviceName);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(OutputEvent_Synced value) synced,
    required TResult Function(OutputEvent_SyncFailed value) syncFailed,
    required TResult Function(OutputEvent_TimelineUpdated value)
        timelineUpdated,
    required TResult Function(OutputEvent_PreAccount value) preAccount,
    required TResult Function(OutputEvent_PostAccount value) postAccount,
    required TResult Function(OutputEvent_DeviceAdded value) deviceAdded,
    required TResult Function(OutputEvent_DocUpdated value) docUpdated,
    required TResult Function(OutputEvent_DownloadCompleted value)
        downloadCompleted,
    required TResult Function(OutputEvent_DownloadFailed value) downloadFailed,
    required TResult Function(OutputEvent_AccUpdated value) accUpdated,
    required TResult Function(OutputEvent_Notification value) notification,
    required TResult Function(OutputEvent_NotificationsUpdated value)
        notificationsUpdated,
    required TResult Function(OutputEvent_LogOut value) logOut,
  }) {
    return deviceAdded(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(OutputEvent_Synced value)? synced,
    TResult? Function(OutputEvent_SyncFailed value)? syncFailed,
    TResult? Function(OutputEvent_TimelineUpdated value)? timelineUpdated,
    TResult? Function(OutputEvent_PreAccount value)? preAccount,
    TResult? Function(OutputEvent_PostAccount value)? postAccount,
    TResult? Function(OutputEvent_DeviceAdded value)? deviceAdded,
    TResult? Function(OutputEvent_DocUpdated value)? docUpdated,
    TResult? Function(OutputEvent_DownloadCompleted value)? downloadCompleted,
    TResult? Function(OutputEvent_DownloadFailed value)? downloadFailed,
    TResult? Function(OutputEvent_AccUpdated value)? accUpdated,
    TResult? Function(OutputEvent_Notification value)? notification,
    TResult? Function(OutputEvent_NotificationsUpdated value)?
        notificationsUpdated,
    TResult? Function(OutputEvent_LogOut value)? logOut,
  }) {
    return deviceAdded?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(OutputEvent_Synced value)? synced,
    TResult Function(OutputEvent_SyncFailed value)? syncFailed,
    TResult Function(OutputEvent_TimelineUpdated value)? timelineUpdated,
    TResult Function(OutputEvent_PreAccount value)? preAccount,
    TResult Function(OutputEvent_PostAccount value)? postAccount,
    TResult Function(OutputEvent_DeviceAdded value)? deviceAdded,
    TResult Function(OutputEvent_DocUpdated value)? docUpdated,
    TResult Function(OutputEvent_DownloadCompleted value)? downloadCompleted,
    TResult Function(OutputEvent_DownloadFailed value)? downloadFailed,
    TResult Function(OutputEvent_AccUpdated value)? accUpdated,
    TResult Function(OutputEvent_Notification value)? notification,
    TResult Function(OutputEvent_NotificationsUpdated value)?
        notificationsUpdated,
    TResult Function(OutputEvent_LogOut value)? logOut,
    required TResult orElse(),
  }) {
    if (deviceAdded != null) {
      return deviceAdded(this);
    }
    return orElse();
  }
}

abstract class OutputEvent_DeviceAdded implements OutputEvent {
  const factory OutputEvent_DeviceAdded({required final String deviceName}) =
      _$OutputEvent_DeviceAdded;

  String get deviceName;
  @JsonKey(ignore: true)
  _$$OutputEvent_DeviceAddedCopyWith<_$OutputEvent_DeviceAdded> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$OutputEvent_DocUpdatedCopyWith<$Res> {
  factory _$$OutputEvent_DocUpdatedCopyWith(_$OutputEvent_DocUpdated value,
          $Res Function(_$OutputEvent_DocUpdated) then) =
      __$$OutputEvent_DocUpdatedCopyWithImpl<$Res>;
  @useResult
  $Res call({String docId});
}

/// @nodoc
class __$$OutputEvent_DocUpdatedCopyWithImpl<$Res>
    extends _$OutputEventCopyWithImpl<$Res, _$OutputEvent_DocUpdated>
    implements _$$OutputEvent_DocUpdatedCopyWith<$Res> {
  __$$OutputEvent_DocUpdatedCopyWithImpl(_$OutputEvent_DocUpdated _value,
      $Res Function(_$OutputEvent_DocUpdated) _then)
      : super(_value, _then);

  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? docId = null,
  }) {
    return _then(_$OutputEvent_DocUpdated(
      docId: null == docId
          ? _value.docId
          : docId // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class _$OutputEvent_DocUpdated implements OutputEvent_DocUpdated {
  const _$OutputEvent_DocUpdated({required this.docId});

  @override
  final String docId;

  @override
  String toString() {
    return 'OutputEvent.docUpdated(docId: $docId)';
  }

  @override
  bool operator ==(dynamic other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$OutputEvent_DocUpdated &&
            (identical(other.docId, docId) || other.docId == docId));
  }

  @override
  int get hashCode => Object.hash(runtimeType, docId);

  @JsonKey(ignore: true)
  @override
  @pragma('vm:prefer-inline')
  _$$OutputEvent_DocUpdatedCopyWith<_$OutputEvent_DocUpdated> get copyWith =>
      __$$OutputEvent_DocUpdatedCopyWithImpl<_$OutputEvent_DocUpdated>(
          this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() synced,
    required TResult Function() syncFailed,
    required TResult Function() timelineUpdated,
    required TResult Function() preAccount,
    required TResult Function(AccView accView) postAccount,
    required TResult Function(String deviceName) deviceAdded,
    required TResult Function(String docId) docUpdated,
    required TResult Function(String blobId, String path) downloadCompleted,
    required TResult Function(String blobId) downloadFailed,
    required TResult Function(AccView field0) accUpdated,
    required TResult Function(String id) notification,
    required TResult Function() notificationsUpdated,
    required TResult Function() logOut,
  }) {
    return docUpdated(docId);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? synced,
    TResult? Function()? syncFailed,
    TResult? Function()? timelineUpdated,
    TResult? Function()? preAccount,
    TResult? Function(AccView accView)? postAccount,
    TResult? Function(String deviceName)? deviceAdded,
    TResult? Function(String docId)? docUpdated,
    TResult? Function(String blobId, String path)? downloadCompleted,
    TResult? Function(String blobId)? downloadFailed,
    TResult? Function(AccView field0)? accUpdated,
    TResult? Function(String id)? notification,
    TResult? Function()? notificationsUpdated,
    TResult? Function()? logOut,
  }) {
    return docUpdated?.call(docId);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? synced,
    TResult Function()? syncFailed,
    TResult Function()? timelineUpdated,
    TResult Function()? preAccount,
    TResult Function(AccView accView)? postAccount,
    TResult Function(String deviceName)? deviceAdded,
    TResult Function(String docId)? docUpdated,
    TResult Function(String blobId, String path)? downloadCompleted,
    TResult Function(String blobId)? downloadFailed,
    TResult Function(AccView field0)? accUpdated,
    TResult Function(String id)? notification,
    TResult Function()? notificationsUpdated,
    TResult Function()? logOut,
    required TResult orElse(),
  }) {
    if (docUpdated != null) {
      return docUpdated(docId);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(OutputEvent_Synced value) synced,
    required TResult Function(OutputEvent_SyncFailed value) syncFailed,
    required TResult Function(OutputEvent_TimelineUpdated value)
        timelineUpdated,
    required TResult Function(OutputEvent_PreAccount value) preAccount,
    required TResult Function(OutputEvent_PostAccount value) postAccount,
    required TResult Function(OutputEvent_DeviceAdded value) deviceAdded,
    required TResult Function(OutputEvent_DocUpdated value) docUpdated,
    required TResult Function(OutputEvent_DownloadCompleted value)
        downloadCompleted,
    required TResult Function(OutputEvent_DownloadFailed value) downloadFailed,
    required TResult Function(OutputEvent_AccUpdated value) accUpdated,
    required TResult Function(OutputEvent_Notification value) notification,
    required TResult Function(OutputEvent_NotificationsUpdated value)
        notificationsUpdated,
    required TResult Function(OutputEvent_LogOut value) logOut,
  }) {
    return docUpdated(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(OutputEvent_Synced value)? synced,
    TResult? Function(OutputEvent_SyncFailed value)? syncFailed,
    TResult? Function(OutputEvent_TimelineUpdated value)? timelineUpdated,
    TResult? Function(OutputEvent_PreAccount value)? preAccount,
    TResult? Function(OutputEvent_PostAccount value)? postAccount,
    TResult? Function(OutputEvent_DeviceAdded value)? deviceAdded,
    TResult? Function(OutputEvent_DocUpdated value)? docUpdated,
    TResult? Function(OutputEvent_DownloadCompleted value)? downloadCompleted,
    TResult? Function(OutputEvent_DownloadFailed value)? downloadFailed,
    TResult? Function(OutputEvent_AccUpdated value)? accUpdated,
    TResult? Function(OutputEvent_Notification value)? notification,
    TResult? Function(OutputEvent_NotificationsUpdated value)?
        notificationsUpdated,
    TResult? Function(OutputEvent_LogOut value)? logOut,
  }) {
    return docUpdated?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(OutputEvent_Synced value)? synced,
    TResult Function(OutputEvent_SyncFailed value)? syncFailed,
    TResult Function(OutputEvent_TimelineUpdated value)? timelineUpdated,
    TResult Function(OutputEvent_PreAccount value)? preAccount,
    TResult Function(OutputEvent_PostAccount value)? postAccount,
    TResult Function(OutputEvent_DeviceAdded value)? deviceAdded,
    TResult Function(OutputEvent_DocUpdated value)? docUpdated,
    TResult Function(OutputEvent_DownloadCompleted value)? downloadCompleted,
    TResult Function(OutputEvent_DownloadFailed value)? downloadFailed,
    TResult Function(OutputEvent_AccUpdated value)? accUpdated,
    TResult Function(OutputEvent_Notification value)? notification,
    TResult Function(OutputEvent_NotificationsUpdated value)?
        notificationsUpdated,
    TResult Function(OutputEvent_LogOut value)? logOut,
    required TResult orElse(),
  }) {
    if (docUpdated != null) {
      return docUpdated(this);
    }
    return orElse();
  }
}

abstract class OutputEvent_DocUpdated implements OutputEvent {
  const factory OutputEvent_DocUpdated({required final String docId}) =
      _$OutputEvent_DocUpdated;

  String get docId;
  @JsonKey(ignore: true)
  _$$OutputEvent_DocUpdatedCopyWith<_$OutputEvent_DocUpdated> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$OutputEvent_DownloadCompletedCopyWith<$Res> {
  factory _$$OutputEvent_DownloadCompletedCopyWith(
          _$OutputEvent_DownloadCompleted value,
          $Res Function(_$OutputEvent_DownloadCompleted) then) =
      __$$OutputEvent_DownloadCompletedCopyWithImpl<$Res>;
  @useResult
  $Res call({String blobId, String path});
}

/// @nodoc
class __$$OutputEvent_DownloadCompletedCopyWithImpl<$Res>
    extends _$OutputEventCopyWithImpl<$Res, _$OutputEvent_DownloadCompleted>
    implements _$$OutputEvent_DownloadCompletedCopyWith<$Res> {
  __$$OutputEvent_DownloadCompletedCopyWithImpl(
      _$OutputEvent_DownloadCompleted _value,
      $Res Function(_$OutputEvent_DownloadCompleted) _then)
      : super(_value, _then);

  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? blobId = null,
    Object? path = null,
  }) {
    return _then(_$OutputEvent_DownloadCompleted(
      blobId: null == blobId
          ? _value.blobId
          : blobId // ignore: cast_nullable_to_non_nullable
              as String,
      path: null == path
          ? _value.path
          : path // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class _$OutputEvent_DownloadCompleted implements OutputEvent_DownloadCompleted {
  const _$OutputEvent_DownloadCompleted(
      {required this.blobId, required this.path});

  @override
  final String blobId;
  @override
  final String path;

  @override
  String toString() {
    return 'OutputEvent.downloadCompleted(blobId: $blobId, path: $path)';
  }

  @override
  bool operator ==(dynamic other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$OutputEvent_DownloadCompleted &&
            (identical(other.blobId, blobId) || other.blobId == blobId) &&
            (identical(other.path, path) || other.path == path));
  }

  @override
  int get hashCode => Object.hash(runtimeType, blobId, path);

  @JsonKey(ignore: true)
  @override
  @pragma('vm:prefer-inline')
  _$$OutputEvent_DownloadCompletedCopyWith<_$OutputEvent_DownloadCompleted>
      get copyWith => __$$OutputEvent_DownloadCompletedCopyWithImpl<
          _$OutputEvent_DownloadCompleted>(this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() synced,
    required TResult Function() syncFailed,
    required TResult Function() timelineUpdated,
    required TResult Function() preAccount,
    required TResult Function(AccView accView) postAccount,
    required TResult Function(String deviceName) deviceAdded,
    required TResult Function(String docId) docUpdated,
    required TResult Function(String blobId, String path) downloadCompleted,
    required TResult Function(String blobId) downloadFailed,
    required TResult Function(AccView field0) accUpdated,
    required TResult Function(String id) notification,
    required TResult Function() notificationsUpdated,
    required TResult Function() logOut,
  }) {
    return downloadCompleted(blobId, path);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? synced,
    TResult? Function()? syncFailed,
    TResult? Function()? timelineUpdated,
    TResult? Function()? preAccount,
    TResult? Function(AccView accView)? postAccount,
    TResult? Function(String deviceName)? deviceAdded,
    TResult? Function(String docId)? docUpdated,
    TResult? Function(String blobId, String path)? downloadCompleted,
    TResult? Function(String blobId)? downloadFailed,
    TResult? Function(AccView field0)? accUpdated,
    TResult? Function(String id)? notification,
    TResult? Function()? notificationsUpdated,
    TResult? Function()? logOut,
  }) {
    return downloadCompleted?.call(blobId, path);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? synced,
    TResult Function()? syncFailed,
    TResult Function()? timelineUpdated,
    TResult Function()? preAccount,
    TResult Function(AccView accView)? postAccount,
    TResult Function(String deviceName)? deviceAdded,
    TResult Function(String docId)? docUpdated,
    TResult Function(String blobId, String path)? downloadCompleted,
    TResult Function(String blobId)? downloadFailed,
    TResult Function(AccView field0)? accUpdated,
    TResult Function(String id)? notification,
    TResult Function()? notificationsUpdated,
    TResult Function()? logOut,
    required TResult orElse(),
  }) {
    if (downloadCompleted != null) {
      return downloadCompleted(blobId, path);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(OutputEvent_Synced value) synced,
    required TResult Function(OutputEvent_SyncFailed value) syncFailed,
    required TResult Function(OutputEvent_TimelineUpdated value)
        timelineUpdated,
    required TResult Function(OutputEvent_PreAccount value) preAccount,
    required TResult Function(OutputEvent_PostAccount value) postAccount,
    required TResult Function(OutputEvent_DeviceAdded value) deviceAdded,
    required TResult Function(OutputEvent_DocUpdated value) docUpdated,
    required TResult Function(OutputEvent_DownloadCompleted value)
        downloadCompleted,
    required TResult Function(OutputEvent_DownloadFailed value) downloadFailed,
    required TResult Function(OutputEvent_AccUpdated value) accUpdated,
    required TResult Function(OutputEvent_Notification value) notification,
    required TResult Function(OutputEvent_NotificationsUpdated value)
        notificationsUpdated,
    required TResult Function(OutputEvent_LogOut value) logOut,
  }) {
    return downloadCompleted(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(OutputEvent_Synced value)? synced,
    TResult? Function(OutputEvent_SyncFailed value)? syncFailed,
    TResult? Function(OutputEvent_TimelineUpdated value)? timelineUpdated,
    TResult? Function(OutputEvent_PreAccount value)? preAccount,
    TResult? Function(OutputEvent_PostAccount value)? postAccount,
    TResult? Function(OutputEvent_DeviceAdded value)? deviceAdded,
    TResult? Function(OutputEvent_DocUpdated value)? docUpdated,
    TResult? Function(OutputEvent_DownloadCompleted value)? downloadCompleted,
    TResult? Function(OutputEvent_DownloadFailed value)? downloadFailed,
    TResult? Function(OutputEvent_AccUpdated value)? accUpdated,
    TResult? Function(OutputEvent_Notification value)? notification,
    TResult? Function(OutputEvent_NotificationsUpdated value)?
        notificationsUpdated,
    TResult? Function(OutputEvent_LogOut value)? logOut,
  }) {
    return downloadCompleted?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(OutputEvent_Synced value)? synced,
    TResult Function(OutputEvent_SyncFailed value)? syncFailed,
    TResult Function(OutputEvent_TimelineUpdated value)? timelineUpdated,
    TResult Function(OutputEvent_PreAccount value)? preAccount,
    TResult Function(OutputEvent_PostAccount value)? postAccount,
    TResult Function(OutputEvent_DeviceAdded value)? deviceAdded,
    TResult Function(OutputEvent_DocUpdated value)? docUpdated,
    TResult Function(OutputEvent_DownloadCompleted value)? downloadCompleted,
    TResult Function(OutputEvent_DownloadFailed value)? downloadFailed,
    TResult Function(OutputEvent_AccUpdated value)? accUpdated,
    TResult Function(OutputEvent_Notification value)? notification,
    TResult Function(OutputEvent_NotificationsUpdated value)?
        notificationsUpdated,
    TResult Function(OutputEvent_LogOut value)? logOut,
    required TResult orElse(),
  }) {
    if (downloadCompleted != null) {
      return downloadCompleted(this);
    }
    return orElse();
  }
}

abstract class OutputEvent_DownloadCompleted implements OutputEvent {
  const factory OutputEvent_DownloadCompleted(
      {required final String blobId,
      required final String path}) = _$OutputEvent_DownloadCompleted;

  String get blobId;
  String get path;
  @JsonKey(ignore: true)
  _$$OutputEvent_DownloadCompletedCopyWith<_$OutputEvent_DownloadCompleted>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$OutputEvent_DownloadFailedCopyWith<$Res> {
  factory _$$OutputEvent_DownloadFailedCopyWith(
          _$OutputEvent_DownloadFailed value,
          $Res Function(_$OutputEvent_DownloadFailed) then) =
      __$$OutputEvent_DownloadFailedCopyWithImpl<$Res>;
  @useResult
  $Res call({String blobId});
}

/// @nodoc
class __$$OutputEvent_DownloadFailedCopyWithImpl<$Res>
    extends _$OutputEventCopyWithImpl<$Res, _$OutputEvent_DownloadFailed>
    implements _$$OutputEvent_DownloadFailedCopyWith<$Res> {
  __$$OutputEvent_DownloadFailedCopyWithImpl(
      _$OutputEvent_DownloadFailed _value,
      $Res Function(_$OutputEvent_DownloadFailed) _then)
      : super(_value, _then);

  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? blobId = null,
  }) {
    return _then(_$OutputEvent_DownloadFailed(
      blobId: null == blobId
          ? _value.blobId
          : blobId // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class _$OutputEvent_DownloadFailed implements OutputEvent_DownloadFailed {
  const _$OutputEvent_DownloadFailed({required this.blobId});

  @override
  final String blobId;

  @override
  String toString() {
    return 'OutputEvent.downloadFailed(blobId: $blobId)';
  }

  @override
  bool operator ==(dynamic other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$OutputEvent_DownloadFailed &&
            (identical(other.blobId, blobId) || other.blobId == blobId));
  }

  @override
  int get hashCode => Object.hash(runtimeType, blobId);

  @JsonKey(ignore: true)
  @override
  @pragma('vm:prefer-inline')
  _$$OutputEvent_DownloadFailedCopyWith<_$OutputEvent_DownloadFailed>
      get copyWith => __$$OutputEvent_DownloadFailedCopyWithImpl<
          _$OutputEvent_DownloadFailed>(this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() synced,
    required TResult Function() syncFailed,
    required TResult Function() timelineUpdated,
    required TResult Function() preAccount,
    required TResult Function(AccView accView) postAccount,
    required TResult Function(String deviceName) deviceAdded,
    required TResult Function(String docId) docUpdated,
    required TResult Function(String blobId, String path) downloadCompleted,
    required TResult Function(String blobId) downloadFailed,
    required TResult Function(AccView field0) accUpdated,
    required TResult Function(String id) notification,
    required TResult Function() notificationsUpdated,
    required TResult Function() logOut,
  }) {
    return downloadFailed(blobId);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? synced,
    TResult? Function()? syncFailed,
    TResult? Function()? timelineUpdated,
    TResult? Function()? preAccount,
    TResult? Function(AccView accView)? postAccount,
    TResult? Function(String deviceName)? deviceAdded,
    TResult? Function(String docId)? docUpdated,
    TResult? Function(String blobId, String path)? downloadCompleted,
    TResult? Function(String blobId)? downloadFailed,
    TResult? Function(AccView field0)? accUpdated,
    TResult? Function(String id)? notification,
    TResult? Function()? notificationsUpdated,
    TResult? Function()? logOut,
  }) {
    return downloadFailed?.call(blobId);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? synced,
    TResult Function()? syncFailed,
    TResult Function()? timelineUpdated,
    TResult Function()? preAccount,
    TResult Function(AccView accView)? postAccount,
    TResult Function(String deviceName)? deviceAdded,
    TResult Function(String docId)? docUpdated,
    TResult Function(String blobId, String path)? downloadCompleted,
    TResult Function(String blobId)? downloadFailed,
    TResult Function(AccView field0)? accUpdated,
    TResult Function(String id)? notification,
    TResult Function()? notificationsUpdated,
    TResult Function()? logOut,
    required TResult orElse(),
  }) {
    if (downloadFailed != null) {
      return downloadFailed(blobId);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(OutputEvent_Synced value) synced,
    required TResult Function(OutputEvent_SyncFailed value) syncFailed,
    required TResult Function(OutputEvent_TimelineUpdated value)
        timelineUpdated,
    required TResult Function(OutputEvent_PreAccount value) preAccount,
    required TResult Function(OutputEvent_PostAccount value) postAccount,
    required TResult Function(OutputEvent_DeviceAdded value) deviceAdded,
    required TResult Function(OutputEvent_DocUpdated value) docUpdated,
    required TResult Function(OutputEvent_DownloadCompleted value)
        downloadCompleted,
    required TResult Function(OutputEvent_DownloadFailed value) downloadFailed,
    required TResult Function(OutputEvent_AccUpdated value) accUpdated,
    required TResult Function(OutputEvent_Notification value) notification,
    required TResult Function(OutputEvent_NotificationsUpdated value)
        notificationsUpdated,
    required TResult Function(OutputEvent_LogOut value) logOut,
  }) {
    return downloadFailed(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(OutputEvent_Synced value)? synced,
    TResult? Function(OutputEvent_SyncFailed value)? syncFailed,
    TResult? Function(OutputEvent_TimelineUpdated value)? timelineUpdated,
    TResult? Function(OutputEvent_PreAccount value)? preAccount,
    TResult? Function(OutputEvent_PostAccount value)? postAccount,
    TResult? Function(OutputEvent_DeviceAdded value)? deviceAdded,
    TResult? Function(OutputEvent_DocUpdated value)? docUpdated,
    TResult? Function(OutputEvent_DownloadCompleted value)? downloadCompleted,
    TResult? Function(OutputEvent_DownloadFailed value)? downloadFailed,
    TResult? Function(OutputEvent_AccUpdated value)? accUpdated,
    TResult? Function(OutputEvent_Notification value)? notification,
    TResult? Function(OutputEvent_NotificationsUpdated value)?
        notificationsUpdated,
    TResult? Function(OutputEvent_LogOut value)? logOut,
  }) {
    return downloadFailed?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(OutputEvent_Synced value)? synced,
    TResult Function(OutputEvent_SyncFailed value)? syncFailed,
    TResult Function(OutputEvent_TimelineUpdated value)? timelineUpdated,
    TResult Function(OutputEvent_PreAccount value)? preAccount,
    TResult Function(OutputEvent_PostAccount value)? postAccount,
    TResult Function(OutputEvent_DeviceAdded value)? deviceAdded,
    TResult Function(OutputEvent_DocUpdated value)? docUpdated,
    TResult Function(OutputEvent_DownloadCompleted value)? downloadCompleted,
    TResult Function(OutputEvent_DownloadFailed value)? downloadFailed,
    TResult Function(OutputEvent_AccUpdated value)? accUpdated,
    TResult Function(OutputEvent_Notification value)? notification,
    TResult Function(OutputEvent_NotificationsUpdated value)?
        notificationsUpdated,
    TResult Function(OutputEvent_LogOut value)? logOut,
    required TResult orElse(),
  }) {
    if (downloadFailed != null) {
      return downloadFailed(this);
    }
    return orElse();
  }
}

abstract class OutputEvent_DownloadFailed implements OutputEvent {
  const factory OutputEvent_DownloadFailed({required final String blobId}) =
      _$OutputEvent_DownloadFailed;

  String get blobId;
  @JsonKey(ignore: true)
  _$$OutputEvent_DownloadFailedCopyWith<_$OutputEvent_DownloadFailed>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$OutputEvent_AccUpdatedCopyWith<$Res> {
  factory _$$OutputEvent_AccUpdatedCopyWith(_$OutputEvent_AccUpdated value,
          $Res Function(_$OutputEvent_AccUpdated) then) =
      __$$OutputEvent_AccUpdatedCopyWithImpl<$Res>;
  @useResult
  $Res call({AccView field0});
}

/// @nodoc
class __$$OutputEvent_AccUpdatedCopyWithImpl<$Res>
    extends _$OutputEventCopyWithImpl<$Res, _$OutputEvent_AccUpdated>
    implements _$$OutputEvent_AccUpdatedCopyWith<$Res> {
  __$$OutputEvent_AccUpdatedCopyWithImpl(_$OutputEvent_AccUpdated _value,
      $Res Function(_$OutputEvent_AccUpdated) _then)
      : super(_value, _then);

  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? field0 = null,
  }) {
    return _then(_$OutputEvent_AccUpdated(
      null == field0
          ? _value.field0
          : field0 // ignore: cast_nullable_to_non_nullable
              as AccView,
    ));
  }
}

/// @nodoc

class _$OutputEvent_AccUpdated implements OutputEvent_AccUpdated {
  const _$OutputEvent_AccUpdated(this.field0);

  @override
  final AccView field0;

  @override
  String toString() {
    return 'OutputEvent.accUpdated(field0: $field0)';
  }

  @override
  bool operator ==(dynamic other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$OutputEvent_AccUpdated &&
            (identical(other.field0, field0) || other.field0 == field0));
  }

  @override
  int get hashCode => Object.hash(runtimeType, field0);

  @JsonKey(ignore: true)
  @override
  @pragma('vm:prefer-inline')
  _$$OutputEvent_AccUpdatedCopyWith<_$OutputEvent_AccUpdated> get copyWith =>
      __$$OutputEvent_AccUpdatedCopyWithImpl<_$OutputEvent_AccUpdated>(
          this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() synced,
    required TResult Function() syncFailed,
    required TResult Function() timelineUpdated,
    required TResult Function() preAccount,
    required TResult Function(AccView accView) postAccount,
    required TResult Function(String deviceName) deviceAdded,
    required TResult Function(String docId) docUpdated,
    required TResult Function(String blobId, String path) downloadCompleted,
    required TResult Function(String blobId) downloadFailed,
    required TResult Function(AccView field0) accUpdated,
    required TResult Function(String id) notification,
    required TResult Function() notificationsUpdated,
    required TResult Function() logOut,
  }) {
    return accUpdated(field0);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? synced,
    TResult? Function()? syncFailed,
    TResult? Function()? timelineUpdated,
    TResult? Function()? preAccount,
    TResult? Function(AccView accView)? postAccount,
    TResult? Function(String deviceName)? deviceAdded,
    TResult? Function(String docId)? docUpdated,
    TResult? Function(String blobId, String path)? downloadCompleted,
    TResult? Function(String blobId)? downloadFailed,
    TResult? Function(AccView field0)? accUpdated,
    TResult? Function(String id)? notification,
    TResult? Function()? notificationsUpdated,
    TResult? Function()? logOut,
  }) {
    return accUpdated?.call(field0);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? synced,
    TResult Function()? syncFailed,
    TResult Function()? timelineUpdated,
    TResult Function()? preAccount,
    TResult Function(AccView accView)? postAccount,
    TResult Function(String deviceName)? deviceAdded,
    TResult Function(String docId)? docUpdated,
    TResult Function(String blobId, String path)? downloadCompleted,
    TResult Function(String blobId)? downloadFailed,
    TResult Function(AccView field0)? accUpdated,
    TResult Function(String id)? notification,
    TResult Function()? notificationsUpdated,
    TResult Function()? logOut,
    required TResult orElse(),
  }) {
    if (accUpdated != null) {
      return accUpdated(field0);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(OutputEvent_Synced value) synced,
    required TResult Function(OutputEvent_SyncFailed value) syncFailed,
    required TResult Function(OutputEvent_TimelineUpdated value)
        timelineUpdated,
    required TResult Function(OutputEvent_PreAccount value) preAccount,
    required TResult Function(OutputEvent_PostAccount value) postAccount,
    required TResult Function(OutputEvent_DeviceAdded value) deviceAdded,
    required TResult Function(OutputEvent_DocUpdated value) docUpdated,
    required TResult Function(OutputEvent_DownloadCompleted value)
        downloadCompleted,
    required TResult Function(OutputEvent_DownloadFailed value) downloadFailed,
    required TResult Function(OutputEvent_AccUpdated value) accUpdated,
    required TResult Function(OutputEvent_Notification value) notification,
    required TResult Function(OutputEvent_NotificationsUpdated value)
        notificationsUpdated,
    required TResult Function(OutputEvent_LogOut value) logOut,
  }) {
    return accUpdated(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(OutputEvent_Synced value)? synced,
    TResult? Function(OutputEvent_SyncFailed value)? syncFailed,
    TResult? Function(OutputEvent_TimelineUpdated value)? timelineUpdated,
    TResult? Function(OutputEvent_PreAccount value)? preAccount,
    TResult? Function(OutputEvent_PostAccount value)? postAccount,
    TResult? Function(OutputEvent_DeviceAdded value)? deviceAdded,
    TResult? Function(OutputEvent_DocUpdated value)? docUpdated,
    TResult? Function(OutputEvent_DownloadCompleted value)? downloadCompleted,
    TResult? Function(OutputEvent_DownloadFailed value)? downloadFailed,
    TResult? Function(OutputEvent_AccUpdated value)? accUpdated,
    TResult? Function(OutputEvent_Notification value)? notification,
    TResult? Function(OutputEvent_NotificationsUpdated value)?
        notificationsUpdated,
    TResult? Function(OutputEvent_LogOut value)? logOut,
  }) {
    return accUpdated?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(OutputEvent_Synced value)? synced,
    TResult Function(OutputEvent_SyncFailed value)? syncFailed,
    TResult Function(OutputEvent_TimelineUpdated value)? timelineUpdated,
    TResult Function(OutputEvent_PreAccount value)? preAccount,
    TResult Function(OutputEvent_PostAccount value)? postAccount,
    TResult Function(OutputEvent_DeviceAdded value)? deviceAdded,
    TResult Function(OutputEvent_DocUpdated value)? docUpdated,
    TResult Function(OutputEvent_DownloadCompleted value)? downloadCompleted,
    TResult Function(OutputEvent_DownloadFailed value)? downloadFailed,
    TResult Function(OutputEvent_AccUpdated value)? accUpdated,
    TResult Function(OutputEvent_Notification value)? notification,
    TResult Function(OutputEvent_NotificationsUpdated value)?
        notificationsUpdated,
    TResult Function(OutputEvent_LogOut value)? logOut,
    required TResult orElse(),
  }) {
    if (accUpdated != null) {
      return accUpdated(this);
    }
    return orElse();
  }
}

abstract class OutputEvent_AccUpdated implements OutputEvent {
  const factory OutputEvent_AccUpdated(final AccView field0) =
      _$OutputEvent_AccUpdated;

  AccView get field0;
  @JsonKey(ignore: true)
  _$$OutputEvent_AccUpdatedCopyWith<_$OutputEvent_AccUpdated> get copyWith =>
      throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$OutputEvent_NotificationCopyWith<$Res> {
  factory _$$OutputEvent_NotificationCopyWith(_$OutputEvent_Notification value,
          $Res Function(_$OutputEvent_Notification) then) =
      __$$OutputEvent_NotificationCopyWithImpl<$Res>;
  @useResult
  $Res call({String id});
}

/// @nodoc
class __$$OutputEvent_NotificationCopyWithImpl<$Res>
    extends _$OutputEventCopyWithImpl<$Res, _$OutputEvent_Notification>
    implements _$$OutputEvent_NotificationCopyWith<$Res> {
  __$$OutputEvent_NotificationCopyWithImpl(_$OutputEvent_Notification _value,
      $Res Function(_$OutputEvent_Notification) _then)
      : super(_value, _then);

  @pragma('vm:prefer-inline')
  @override
  $Res call({
    Object? id = null,
  }) {
    return _then(_$OutputEvent_Notification(
      id: null == id
          ? _value.id
          : id // ignore: cast_nullable_to_non_nullable
              as String,
    ));
  }
}

/// @nodoc

class _$OutputEvent_Notification implements OutputEvent_Notification {
  const _$OutputEvent_Notification({required this.id});

  @override
  final String id;

  @override
  String toString() {
    return 'OutputEvent.notification(id: $id)';
  }

  @override
  bool operator ==(dynamic other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$OutputEvent_Notification &&
            (identical(other.id, id) || other.id == id));
  }

  @override
  int get hashCode => Object.hash(runtimeType, id);

  @JsonKey(ignore: true)
  @override
  @pragma('vm:prefer-inline')
  _$$OutputEvent_NotificationCopyWith<_$OutputEvent_Notification>
      get copyWith =>
          __$$OutputEvent_NotificationCopyWithImpl<_$OutputEvent_Notification>(
              this, _$identity);

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() synced,
    required TResult Function() syncFailed,
    required TResult Function() timelineUpdated,
    required TResult Function() preAccount,
    required TResult Function(AccView accView) postAccount,
    required TResult Function(String deviceName) deviceAdded,
    required TResult Function(String docId) docUpdated,
    required TResult Function(String blobId, String path) downloadCompleted,
    required TResult Function(String blobId) downloadFailed,
    required TResult Function(AccView field0) accUpdated,
    required TResult Function(String id) notification,
    required TResult Function() notificationsUpdated,
    required TResult Function() logOut,
  }) {
    return notification(id);
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? synced,
    TResult? Function()? syncFailed,
    TResult? Function()? timelineUpdated,
    TResult? Function()? preAccount,
    TResult? Function(AccView accView)? postAccount,
    TResult? Function(String deviceName)? deviceAdded,
    TResult? Function(String docId)? docUpdated,
    TResult? Function(String blobId, String path)? downloadCompleted,
    TResult? Function(String blobId)? downloadFailed,
    TResult? Function(AccView field0)? accUpdated,
    TResult? Function(String id)? notification,
    TResult? Function()? notificationsUpdated,
    TResult? Function()? logOut,
  }) {
    return notification?.call(id);
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? synced,
    TResult Function()? syncFailed,
    TResult Function()? timelineUpdated,
    TResult Function()? preAccount,
    TResult Function(AccView accView)? postAccount,
    TResult Function(String deviceName)? deviceAdded,
    TResult Function(String docId)? docUpdated,
    TResult Function(String blobId, String path)? downloadCompleted,
    TResult Function(String blobId)? downloadFailed,
    TResult Function(AccView field0)? accUpdated,
    TResult Function(String id)? notification,
    TResult Function()? notificationsUpdated,
    TResult Function()? logOut,
    required TResult orElse(),
  }) {
    if (notification != null) {
      return notification(id);
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(OutputEvent_Synced value) synced,
    required TResult Function(OutputEvent_SyncFailed value) syncFailed,
    required TResult Function(OutputEvent_TimelineUpdated value)
        timelineUpdated,
    required TResult Function(OutputEvent_PreAccount value) preAccount,
    required TResult Function(OutputEvent_PostAccount value) postAccount,
    required TResult Function(OutputEvent_DeviceAdded value) deviceAdded,
    required TResult Function(OutputEvent_DocUpdated value) docUpdated,
    required TResult Function(OutputEvent_DownloadCompleted value)
        downloadCompleted,
    required TResult Function(OutputEvent_DownloadFailed value) downloadFailed,
    required TResult Function(OutputEvent_AccUpdated value) accUpdated,
    required TResult Function(OutputEvent_Notification value) notification,
    required TResult Function(OutputEvent_NotificationsUpdated value)
        notificationsUpdated,
    required TResult Function(OutputEvent_LogOut value) logOut,
  }) {
    return notification(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(OutputEvent_Synced value)? synced,
    TResult? Function(OutputEvent_SyncFailed value)? syncFailed,
    TResult? Function(OutputEvent_TimelineUpdated value)? timelineUpdated,
    TResult? Function(OutputEvent_PreAccount value)? preAccount,
    TResult? Function(OutputEvent_PostAccount value)? postAccount,
    TResult? Function(OutputEvent_DeviceAdded value)? deviceAdded,
    TResult? Function(OutputEvent_DocUpdated value)? docUpdated,
    TResult? Function(OutputEvent_DownloadCompleted value)? downloadCompleted,
    TResult? Function(OutputEvent_DownloadFailed value)? downloadFailed,
    TResult? Function(OutputEvent_AccUpdated value)? accUpdated,
    TResult? Function(OutputEvent_Notification value)? notification,
    TResult? Function(OutputEvent_NotificationsUpdated value)?
        notificationsUpdated,
    TResult? Function(OutputEvent_LogOut value)? logOut,
  }) {
    return notification?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(OutputEvent_Synced value)? synced,
    TResult Function(OutputEvent_SyncFailed value)? syncFailed,
    TResult Function(OutputEvent_TimelineUpdated value)? timelineUpdated,
    TResult Function(OutputEvent_PreAccount value)? preAccount,
    TResult Function(OutputEvent_PostAccount value)? postAccount,
    TResult Function(OutputEvent_DeviceAdded value)? deviceAdded,
    TResult Function(OutputEvent_DocUpdated value)? docUpdated,
    TResult Function(OutputEvent_DownloadCompleted value)? downloadCompleted,
    TResult Function(OutputEvent_DownloadFailed value)? downloadFailed,
    TResult Function(OutputEvent_AccUpdated value)? accUpdated,
    TResult Function(OutputEvent_Notification value)? notification,
    TResult Function(OutputEvent_NotificationsUpdated value)?
        notificationsUpdated,
    TResult Function(OutputEvent_LogOut value)? logOut,
    required TResult orElse(),
  }) {
    if (notification != null) {
      return notification(this);
    }
    return orElse();
  }
}

abstract class OutputEvent_Notification implements OutputEvent {
  const factory OutputEvent_Notification({required final String id}) =
      _$OutputEvent_Notification;

  String get id;
  @JsonKey(ignore: true)
  _$$OutputEvent_NotificationCopyWith<_$OutputEvent_Notification>
      get copyWith => throw _privateConstructorUsedError;
}

/// @nodoc
abstract class _$$OutputEvent_NotificationsUpdatedCopyWith<$Res> {
  factory _$$OutputEvent_NotificationsUpdatedCopyWith(
          _$OutputEvent_NotificationsUpdated value,
          $Res Function(_$OutputEvent_NotificationsUpdated) then) =
      __$$OutputEvent_NotificationsUpdatedCopyWithImpl<$Res>;
}

/// @nodoc
class __$$OutputEvent_NotificationsUpdatedCopyWithImpl<$Res>
    extends _$OutputEventCopyWithImpl<$Res, _$OutputEvent_NotificationsUpdated>
    implements _$$OutputEvent_NotificationsUpdatedCopyWith<$Res> {
  __$$OutputEvent_NotificationsUpdatedCopyWithImpl(
      _$OutputEvent_NotificationsUpdated _value,
      $Res Function(_$OutputEvent_NotificationsUpdated) _then)
      : super(_value, _then);
}

/// @nodoc

class _$OutputEvent_NotificationsUpdated
    implements OutputEvent_NotificationsUpdated {
  const _$OutputEvent_NotificationsUpdated();

  @override
  String toString() {
    return 'OutputEvent.notificationsUpdated()';
  }

  @override
  bool operator ==(dynamic other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType &&
            other is _$OutputEvent_NotificationsUpdated);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() synced,
    required TResult Function() syncFailed,
    required TResult Function() timelineUpdated,
    required TResult Function() preAccount,
    required TResult Function(AccView accView) postAccount,
    required TResult Function(String deviceName) deviceAdded,
    required TResult Function(String docId) docUpdated,
    required TResult Function(String blobId, String path) downloadCompleted,
    required TResult Function(String blobId) downloadFailed,
    required TResult Function(AccView field0) accUpdated,
    required TResult Function(String id) notification,
    required TResult Function() notificationsUpdated,
    required TResult Function() logOut,
  }) {
    return notificationsUpdated();
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? synced,
    TResult? Function()? syncFailed,
    TResult? Function()? timelineUpdated,
    TResult? Function()? preAccount,
    TResult? Function(AccView accView)? postAccount,
    TResult? Function(String deviceName)? deviceAdded,
    TResult? Function(String docId)? docUpdated,
    TResult? Function(String blobId, String path)? downloadCompleted,
    TResult? Function(String blobId)? downloadFailed,
    TResult? Function(AccView field0)? accUpdated,
    TResult? Function(String id)? notification,
    TResult? Function()? notificationsUpdated,
    TResult? Function()? logOut,
  }) {
    return notificationsUpdated?.call();
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? synced,
    TResult Function()? syncFailed,
    TResult Function()? timelineUpdated,
    TResult Function()? preAccount,
    TResult Function(AccView accView)? postAccount,
    TResult Function(String deviceName)? deviceAdded,
    TResult Function(String docId)? docUpdated,
    TResult Function(String blobId, String path)? downloadCompleted,
    TResult Function(String blobId)? downloadFailed,
    TResult Function(AccView field0)? accUpdated,
    TResult Function(String id)? notification,
    TResult Function()? notificationsUpdated,
    TResult Function()? logOut,
    required TResult orElse(),
  }) {
    if (notificationsUpdated != null) {
      return notificationsUpdated();
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(OutputEvent_Synced value) synced,
    required TResult Function(OutputEvent_SyncFailed value) syncFailed,
    required TResult Function(OutputEvent_TimelineUpdated value)
        timelineUpdated,
    required TResult Function(OutputEvent_PreAccount value) preAccount,
    required TResult Function(OutputEvent_PostAccount value) postAccount,
    required TResult Function(OutputEvent_DeviceAdded value) deviceAdded,
    required TResult Function(OutputEvent_DocUpdated value) docUpdated,
    required TResult Function(OutputEvent_DownloadCompleted value)
        downloadCompleted,
    required TResult Function(OutputEvent_DownloadFailed value) downloadFailed,
    required TResult Function(OutputEvent_AccUpdated value) accUpdated,
    required TResult Function(OutputEvent_Notification value) notification,
    required TResult Function(OutputEvent_NotificationsUpdated value)
        notificationsUpdated,
    required TResult Function(OutputEvent_LogOut value) logOut,
  }) {
    return notificationsUpdated(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(OutputEvent_Synced value)? synced,
    TResult? Function(OutputEvent_SyncFailed value)? syncFailed,
    TResult? Function(OutputEvent_TimelineUpdated value)? timelineUpdated,
    TResult? Function(OutputEvent_PreAccount value)? preAccount,
    TResult? Function(OutputEvent_PostAccount value)? postAccount,
    TResult? Function(OutputEvent_DeviceAdded value)? deviceAdded,
    TResult? Function(OutputEvent_DocUpdated value)? docUpdated,
    TResult? Function(OutputEvent_DownloadCompleted value)? downloadCompleted,
    TResult? Function(OutputEvent_DownloadFailed value)? downloadFailed,
    TResult? Function(OutputEvent_AccUpdated value)? accUpdated,
    TResult? Function(OutputEvent_Notification value)? notification,
    TResult? Function(OutputEvent_NotificationsUpdated value)?
        notificationsUpdated,
    TResult? Function(OutputEvent_LogOut value)? logOut,
  }) {
    return notificationsUpdated?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(OutputEvent_Synced value)? synced,
    TResult Function(OutputEvent_SyncFailed value)? syncFailed,
    TResult Function(OutputEvent_TimelineUpdated value)? timelineUpdated,
    TResult Function(OutputEvent_PreAccount value)? preAccount,
    TResult Function(OutputEvent_PostAccount value)? postAccount,
    TResult Function(OutputEvent_DeviceAdded value)? deviceAdded,
    TResult Function(OutputEvent_DocUpdated value)? docUpdated,
    TResult Function(OutputEvent_DownloadCompleted value)? downloadCompleted,
    TResult Function(OutputEvent_DownloadFailed value)? downloadFailed,
    TResult Function(OutputEvent_AccUpdated value)? accUpdated,
    TResult Function(OutputEvent_Notification value)? notification,
    TResult Function(OutputEvent_NotificationsUpdated value)?
        notificationsUpdated,
    TResult Function(OutputEvent_LogOut value)? logOut,
    required TResult orElse(),
  }) {
    if (notificationsUpdated != null) {
      return notificationsUpdated(this);
    }
    return orElse();
  }
}

abstract class OutputEvent_NotificationsUpdated implements OutputEvent {
  const factory OutputEvent_NotificationsUpdated() =
      _$OutputEvent_NotificationsUpdated;
}

/// @nodoc
abstract class _$$OutputEvent_LogOutCopyWith<$Res> {
  factory _$$OutputEvent_LogOutCopyWith(_$OutputEvent_LogOut value,
          $Res Function(_$OutputEvent_LogOut) then) =
      __$$OutputEvent_LogOutCopyWithImpl<$Res>;
}

/// @nodoc
class __$$OutputEvent_LogOutCopyWithImpl<$Res>
    extends _$OutputEventCopyWithImpl<$Res, _$OutputEvent_LogOut>
    implements _$$OutputEvent_LogOutCopyWith<$Res> {
  __$$OutputEvent_LogOutCopyWithImpl(
      _$OutputEvent_LogOut _value, $Res Function(_$OutputEvent_LogOut) _then)
      : super(_value, _then);
}

/// @nodoc

class _$OutputEvent_LogOut implements OutputEvent_LogOut {
  const _$OutputEvent_LogOut();

  @override
  String toString() {
    return 'OutputEvent.logOut()';
  }

  @override
  bool operator ==(dynamic other) {
    return identical(this, other) ||
        (other.runtimeType == runtimeType && other is _$OutputEvent_LogOut);
  }

  @override
  int get hashCode => runtimeType.hashCode;

  @override
  @optionalTypeArgs
  TResult when<TResult extends Object?>({
    required TResult Function() synced,
    required TResult Function() syncFailed,
    required TResult Function() timelineUpdated,
    required TResult Function() preAccount,
    required TResult Function(AccView accView) postAccount,
    required TResult Function(String deviceName) deviceAdded,
    required TResult Function(String docId) docUpdated,
    required TResult Function(String blobId, String path) downloadCompleted,
    required TResult Function(String blobId) downloadFailed,
    required TResult Function(AccView field0) accUpdated,
    required TResult Function(String id) notification,
    required TResult Function() notificationsUpdated,
    required TResult Function() logOut,
  }) {
    return logOut();
  }

  @override
  @optionalTypeArgs
  TResult? whenOrNull<TResult extends Object?>({
    TResult? Function()? synced,
    TResult? Function()? syncFailed,
    TResult? Function()? timelineUpdated,
    TResult? Function()? preAccount,
    TResult? Function(AccView accView)? postAccount,
    TResult? Function(String deviceName)? deviceAdded,
    TResult? Function(String docId)? docUpdated,
    TResult? Function(String blobId, String path)? downloadCompleted,
    TResult? Function(String blobId)? downloadFailed,
    TResult? Function(AccView field0)? accUpdated,
    TResult? Function(String id)? notification,
    TResult? Function()? notificationsUpdated,
    TResult? Function()? logOut,
  }) {
    return logOut?.call();
  }

  @override
  @optionalTypeArgs
  TResult maybeWhen<TResult extends Object?>({
    TResult Function()? synced,
    TResult Function()? syncFailed,
    TResult Function()? timelineUpdated,
    TResult Function()? preAccount,
    TResult Function(AccView accView)? postAccount,
    TResult Function(String deviceName)? deviceAdded,
    TResult Function(String docId)? docUpdated,
    TResult Function(String blobId, String path)? downloadCompleted,
    TResult Function(String blobId)? downloadFailed,
    TResult Function(AccView field0)? accUpdated,
    TResult Function(String id)? notification,
    TResult Function()? notificationsUpdated,
    TResult Function()? logOut,
    required TResult orElse(),
  }) {
    if (logOut != null) {
      return logOut();
    }
    return orElse();
  }

  @override
  @optionalTypeArgs
  TResult map<TResult extends Object?>({
    required TResult Function(OutputEvent_Synced value) synced,
    required TResult Function(OutputEvent_SyncFailed value) syncFailed,
    required TResult Function(OutputEvent_TimelineUpdated value)
        timelineUpdated,
    required TResult Function(OutputEvent_PreAccount value) preAccount,
    required TResult Function(OutputEvent_PostAccount value) postAccount,
    required TResult Function(OutputEvent_DeviceAdded value) deviceAdded,
    required TResult Function(OutputEvent_DocUpdated value) docUpdated,
    required TResult Function(OutputEvent_DownloadCompleted value)
        downloadCompleted,
    required TResult Function(OutputEvent_DownloadFailed value) downloadFailed,
    required TResult Function(OutputEvent_AccUpdated value) accUpdated,
    required TResult Function(OutputEvent_Notification value) notification,
    required TResult Function(OutputEvent_NotificationsUpdated value)
        notificationsUpdated,
    required TResult Function(OutputEvent_LogOut value) logOut,
  }) {
    return logOut(this);
  }

  @override
  @optionalTypeArgs
  TResult? mapOrNull<TResult extends Object?>({
    TResult? Function(OutputEvent_Synced value)? synced,
    TResult? Function(OutputEvent_SyncFailed value)? syncFailed,
    TResult? Function(OutputEvent_TimelineUpdated value)? timelineUpdated,
    TResult? Function(OutputEvent_PreAccount value)? preAccount,
    TResult? Function(OutputEvent_PostAccount value)? postAccount,
    TResult? Function(OutputEvent_DeviceAdded value)? deviceAdded,
    TResult? Function(OutputEvent_DocUpdated value)? docUpdated,
    TResult? Function(OutputEvent_DownloadCompleted value)? downloadCompleted,
    TResult? Function(OutputEvent_DownloadFailed value)? downloadFailed,
    TResult? Function(OutputEvent_AccUpdated value)? accUpdated,
    TResult? Function(OutputEvent_Notification value)? notification,
    TResult? Function(OutputEvent_NotificationsUpdated value)?
        notificationsUpdated,
    TResult? Function(OutputEvent_LogOut value)? logOut,
  }) {
    return logOut?.call(this);
  }

  @override
  @optionalTypeArgs
  TResult maybeMap<TResult extends Object?>({
    TResult Function(OutputEvent_Synced value)? synced,
    TResult Function(OutputEvent_SyncFailed value)? syncFailed,
    TResult Function(OutputEvent_TimelineUpdated value)? timelineUpdated,
    TResult Function(OutputEvent_PreAccount value)? preAccount,
    TResult Function(OutputEvent_PostAccount value)? postAccount,
    TResult Function(OutputEvent_DeviceAdded value)? deviceAdded,
    TResult Function(OutputEvent_DocUpdated value)? docUpdated,
    TResult Function(OutputEvent_DownloadCompleted value)? downloadCompleted,
    TResult Function(OutputEvent_DownloadFailed value)? downloadFailed,
    TResult Function(OutputEvent_AccUpdated value)? accUpdated,
    TResult Function(OutputEvent_Notification value)? notification,
    TResult Function(OutputEvent_NotificationsUpdated value)?
        notificationsUpdated,
    TResult Function(OutputEvent_LogOut value)? logOut,
    required TResult orElse(),
  }) {
    if (logOut != null) {
      return logOut(this);
    }
    return orElse();
  }
}

abstract class OutputEvent_LogOut implements OutputEvent {
  const factory OutputEvent_LogOut() = _$OutputEvent_LogOut;
}
