#!/usr/bin/env bash
# Builds the Android app: the Rust engine for each phone architecture
# (cargo-ndk), the app's name from the content pack, then Gradle.
#   needs: Rust with the Android targets, cargo-ndk, an Android SDK and NDK
#   (ANDROID_HOME, ANDROID_NDK_HOME), Java 17+, Node.
#   output: android/dist/scraped-again.apk
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
echo "android/dist/scraped-again.apk"
