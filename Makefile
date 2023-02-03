.DEFAULT_GOAL := help

.PHONY: help
# From: http://disq.us/p/16327nq
help: ## This help.
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / {printf "\033[36m%-30s\033[0m %s\n", $$1, $$2}' $(MAKEFILE_LIST)


.PHONY: codegen
codegen: ## Generate Rust and Dart glue code
	flutter_rust_bridge_codegen \
		-r native/src/api.rs \
		-d lib/bridge_generated.dart \
		-c ios/Runner/bridge_generated.h \
		-e macos/Runner/

.PHONY: flutter-upgrade-deps
flutter-upgrade-deps: ## Upgrade Flutter dependencies
	(cd app && flutter pub upgrade --major-versions)
	(cd app && flutter pub upgrade)

.PHONY: flutter-run-linux
flutter-run-linux: ## Run Flutter app on Linux host
	(cd app && flutter run -d linux)

.PHONY: flutter-run-linux-alt
flutter-run-linux-alt: ## Run Flutter app on Linux host (different account)
	(cd app && flutter run --dart-define=BOLIK_APP_SUPPORT_PATH=${HOME}/.local/share/tech.bolik.timeline-alt -d linux)

.PHONY: flutter-run-mac
flutter-run-mac: ## Run Flutter app on Mac host
	(cd app && flutter run -d macos)

.PHONY: flutter-run-mac-alt
flutter-run-mac-alt: ## Run Flutter app on Mac host (different account)
	(cd app && flutter run --dart-define=BOLIK_APP_SUPPORT_PATH=${HOME}/Library/Containers/tech.bolik.timelineapp/Data/Library/Application\ Support/tech.bolik.timeline-alt -d macos)

.PHONY: deploy
deploy: ## Deploy bolik server
	echo "Deploying bolik server..."
	fly deploy --config fly-bolik-api.toml --dockerfile Dockerfile.bolik-api -r waw

.PHONY: logs
logs: ## Tail server logs
	fly logs -a bolik-api

.PHONY: server-run
server-run: export S3_BUCKET=bolik-bucket
server-run: export S3_ENDPOINT=s3.localhost
server-run: export AWS_ACCESS_KEY_ID=key-id
server-run: export AWS_SECRET_ACCESS_KEY=access-key
server-run: ## Run local server
	(cd bolik_server && cargo run)
