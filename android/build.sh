#!/usr/bin/env bash
# Builds the Android app: the Rust engine for each phone architecture
# (cargo-ndk), the app's name from the content pack, then Gradle.
#   needs: Rust with the Android targets, cargo-ndk, an Android SDK and NDK
#   (ANDROID_HOME, ANDROID_NDK_HOME), Java 17+, Node.
#   output: android/dist/scraped-again.apk (every phone architecture and
#   x86_64 emulators) and android/dist/scraped-again-arm64.apk (64-bit ARM
#   phones only: small enough to share in a chat app)
# Set ANDROID_KEYSTORE (a .p12 file), ANDROID_KEYSTORE_PASSWORD and
# optionally ANDROID_KEY_ALIAS to sign for release; otherwise the debug key.
set -euo pipefail
cd "$(dirname "$0")/.."

tools/build.sh
mkdir -p android/app/build/generated/labels/res/values
node tools/smoke/labels.cjs android/app/build/generated/labels/res/values/strings.xml

cargo ndk -t arm64-v8a -t armeabi-v7a -t x86_64 -o android/app/src/main/jniLibs \
  build --release -p scraped-android
# Only the app's own library: cargo-ndk also copies the web crate's.
rm -f android/app/src/main/jniLibs/*/libscraped_web.so

cd android
./gradlew --no-daemon -q assembleRelease "$@"
mkdir -p dist
cp app/build/outputs/apk/release/app-release.apk dist/scraped-again.apk

# Again with only the 64-bit ARM engine: every phone of recent years, at
# about a third of the native code. The other architectures are put back
# afterwards for the emulator tests.
held=build/other-abis
rm -rf "$held" && mkdir -p "$held"
for abi in app/src/main/jniLibs/*/; do
  [ "$(basename "$abi")" = arm64-v8a ] || mv "$abi" "$held/"
done
./gradlew --no-daemon -q assembleRelease "$@"
cp app/build/outputs/apk/release/app-release.apk dist/scraped-again-arm64.apk
mv "$held"/* app/src/main/jniLibs/
rmdir "$held"

for f in dist/scraped-again.apk dist/scraped-again-arm64.apk; do
  echo "android/$f ($(du -k "$f" | cut -f1) KB)"
done
