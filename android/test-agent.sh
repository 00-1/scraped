#!/usr/bin/env bash
# Off-device tests of the agent server and protocol (plain Java, no SDK).
#   needs: a JDK and the org.json jar (JSON_JAR, or fetched from Maven Central)
set -euo pipefail
cd "$(dirname "$0")"
out=build/agent-test
mkdir -p "$out"
jar="${JSON_JAR:-$out/json.jar}"
[ -f "$jar" ] || curl -sSfL -o "$jar" https://repo1.maven.org/maven2/org/json/json/20240303/json-20240303.jar
javac -d "$out" -cp "$jar" app/src/main/java/org/scrapedagain/AgentProtocol.java \
  app/src/main/java/org/scrapedagain/AgentServer.java test/AgentTest.java
java -cp "$out:$jar" AgentTest
