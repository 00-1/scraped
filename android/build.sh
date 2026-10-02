#!/usr/bin/env bash
# Builds the Android app (android/dist/scraped-again.apk) without Gradle or
# the Android SDK manager: the platform jar, aapt2, D8 and apksig are
# fetched once into android/.tools. Needs a JDK (17+), node, python3 and
# what tools/build.sh needs (Rust with the wasm32 target).
#
#   android/build.sh            build, test, sign
#   ANDROID_KEYSTORE=… ANDROID_KEYSTORE_PASSWORD=…   sign with your key
#
# Without a keystore a local one is made in android/.keystore (keep it:
# Android only updates an app signed with the same key).
set -euo pipefail
cd "$(dirname "$0")"
ROOT=$(cd .. && pwd)
T=.tools
B=build
mkdir -p "$T" "$B" dist

fetch() { # url file
  [ -s "$T/$2" ] && return 0
  echo "fetching $2"
  curl -fsSL --retry 3 -o "$T/$2.part" "$1" && mv "$T/$2.part" "$T/$2"
}
fetch https://raw.githubusercontent.com/Sable/android-platforms/master/android-35/android.jar android-35.jar
fetch https://storage.googleapis.com/r8-releases/raw/8.5.35/r8lib.jar r8.jar
fetch https://repo1.maven.org/maven2/com/android/tools/build/apksig/2.3.0/apksig-2.3.0.jar apksig.jar
fetch https://repo1.maven.org/maven2/org/json/json/20240303/json-20240303.jar json.jar
if [ ! -x "$T/aapt2" ]; then
  fetch https://github.com/iBotPeaches/Apktool/releases/download/v2.10.0/apktool_2.10.0.jar apktool.jar
  case "$(uname -s)" in
    Linux) inner=prebuilt/linux/aapt2_64 ;;
    Darwin) inner=prebuilt/macosx/aapt2_64 ;;
    *) echo "build on Linux or macOS" >&2; exit 1 ;;
  esac
  (cd "$T" && unzip -o -q -j apktool.jar "$inner" && mv "$(basename "$inner")" aapt2 && chmod +x aapt2)
fi
JAR=$T/android-35.jar

# The page, with the engine and Jb's content inside it.
"$ROOT/tools/build.sh" >/dev/null
rm -rf "$B"
mkdir -p "$B/assets" "$B/res/values" "$B/gen" "$B/classes" "$B/dex" "$B/test"
cp "$ROOT/tools/dist/app.html" "$B/assets/index.html"

# The app's name, from Jb's app.label slot.
node "$ROOT/tools/smoke/labels.cjs" "$B/res/values/strings.xml"

# Resources and manifest.
VERSION_NAME=$(sed -n 's/^version = "\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)
VERSION_CODE=$(git -C "$ROOT" rev-list --count HEAD 2>/dev/null || echo 1)
"$T/aapt2" compile --dir res -o "$B/res-main.zip"
"$T/aapt2" compile --dir "$B/res" -o "$B/res-gen.zip"
"$T/aapt2" link -o "$B/base.apk" -I "$JAR" --manifest AndroidManifest.xml \
  --min-sdk-version 26 --target-sdk-version 35 \
  --version-code "$VERSION_CODE" --version-name "$VERSION_NAME" \
  -A "$B/assets" --java "$B/gen" --auto-add-overlay "$B/res-main.zip" "$B/res-gen.zip"

# Code: Java 8 against the platform, then dex.
javac -nowarn -Xlint:-options -source 8 -target 8 -encoding UTF-8 -bootclasspath "$JAR" \
  -d "$B/classes" $(find src "$B/gen" -name '*.java')
java -cp "$T/r8.jar" com.android.tools.r8.D8 --release --min-api 26 --lib "$JAR" \
  --output "$B/dex" $(find "$B/classes" -name '*.class')

# The agent server's tests, off the device.
javac -nowarn -encoding UTF-8 -cp "$T/json.jar" -d "$B/test" \
  src/org/scrapedagain/AgentProtocol.java src/org/scrapedagain/AgentServer.java test/AgentTest.java
java -cp "$B/test:$T/json.jar" AgentTest

# Package and sign.
cp "$B/base.apk" "$B/unsigned.apk"
(cd "$B/dex" && zip -q -X ../unsigned.apk classes.dex)
KS=${ANDROID_KEYSTORE:-.keystore/scraped-again.p12}
PW=${ANDROID_KEYSTORE_PASSWORD:-scraped-again}
ALIAS=${ANDROID_KEY_ALIAS:-scraped}
if [ ! -f "$KS" ]; then
  mkdir -p "$(dirname "$KS")"
  keytool -genkeypair -keystore "$KS" -storetype PKCS12 -storepass "$PW" -keypass "$PW" \
    -alias "$ALIAS" -keyalg RSA -keysize 3072 -validity 10000 -dname "CN=Scraped Again" >/dev/null 2>&1
  echo "made a new signing key in $KS (keep it to update the app later)"
fi
java --add-exports java.base/sun.security.x509=ALL-UNNAMED --add-exports java.base/sun.security.pkcs=ALL-UNNAMED --add-exports java.base/sun.security.util=ALL-UNNAMED \
  -cp "$T/apksig.jar" tools/Sign.java "$B/unsigned.apk" dist/scraped-again.apk "$KS" "$PW" "$ALIAS"
"$T/aapt2" dump badging dist/scraped-again.apk | head -3
ls -l dist/scraped-again.apk
