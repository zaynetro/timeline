# Bolik monorepo

Bolik Timeline is [local-first](https://www.inkandswitch.com/local-first/) software for keeping notes and files.

> This repo contains **alpha-quality software**. This means that we are expecting breaking changes and potential data loss.


## Features

Bolik Timeline is an application for managing personal documents like notes, photos and memories. It supports offline editing, is end-to-end encrypted and is open source unlike other popular solutions. Timeline is an attempt to organize your private life under a single app.

* A chronological timeline of your notes and files.
* End-to-end encryption with [OpenMLS](https://github.com/openmls/openmls) and ChaCha20Poly1305.
* Offline editing support by leveraging [Yrs](https://github.com/y-crdt/y-crdt/) CRDT.
* Multi-device synchronization.
* Selective sharing: Share only the content you want while keeping the rest private and in the same place.
* Access your data from every major operating system: Android, iOS, Mac and Linux (Windows planned).
* No lock-in. Fully open source. At any time you can export your data to Markdown files.

Read an [introductory blog post](https://www.zaynetro.com/post/how-to-build-e2ee-local-first-app/) for more details.

## How to install?

You can join an open beta on Android, iOS and Mac.

* iOS and Mac: <https://testflight.apple.com/join/C6RWPhFR>
* Android: <https://play.google.com/store/apps/details?id=tech.bolik.timeline>

For Linux you will need to build yourself. Windows is not supported for the time being.

### Run yourself

* Install Rust nightly and Flutter
* `cd app && flutter run`


## Repo structure

* `app`: Cross-platform Flutter application.
* `app/native`: FFI module, bridge Flutter with SDK.
* `bolik_chain`: Signature chain.
* `bolik_proto`: Protobuf definitions.
* `bolik_sdk`: Client SDK.
* `bolik_server`: Server.
* `bolik_tests`: Integration tests.


## License

Flutter application (`app`) and Flutter FFI module (`app/native`) are released under GPL license. All other projects are MIT licensed.
