# The engine is reached through JNI by name.
-keep class org.scrapedagain.Engine { native <methods>; *; }
# The agent server's protocol uses org.json reflection-free; keep it whole for clarity.
-keep class org.scrapedagain.AgentProtocol** { *; }
