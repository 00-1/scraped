// The Android app: Kotlin and Jetpack Compose for everything on screen; the
// game itself is the Rust engine (crates/android), built by cargo-ndk into
// src/main/jniLibs before Gradle runs (see android/build.sh).
import java.util.Properties

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

fun gitCount(): Int = try {
    val p = ProcessBuilder("git", "rev-list", "--count", "HEAD").redirectErrorStream(true).start()
    p.inputStream.bufferedReader().readText().trim().toInt()
} catch (e: Exception) { 1 }

android {
    namespace = "org.scrapedagain"
    compileSdk = 35

    defaultConfig {
        applicationId = "org.scrapedagain"
        minSdk = 26
        targetSdk = 35
        versionCode = gitCount()
        versionName = "0.2.0"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    // The app's name comes from Jb's app.label slot, written by build.sh.
    sourceSets["main"].res.srcDir("build/generated/labels/res")
    // The cross-platform transcripts, for the on-device determinism test.
    sourceSets["androidTest"].assets.srcDir("../../crates/game/tests")

    signingConfigs {
        create("release") {
            val ks = System.getenv("ANDROID_KEYSTORE")
            if (ks != null && file(ks).exists()) {
                storeFile = file(ks)
                storePassword = System.getenv("ANDROID_KEYSTORE_PASSWORD") ?: ""
                keyAlias = System.getenv("ANDROID_KEY_ALIAS") ?: "scraped"
                keyPassword = System.getenv("ANDROID_KEYSTORE_PASSWORD") ?: ""
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            signingConfig = if (System.getenv("ANDROID_KEYSTORE") != null) {
                signingConfigs.getByName("release")
            } else {
                signingConfigs.getByName("debug")
            }
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }
    buildFeatures {
        compose = true
    }
    packaging {
        jniLibs.useLegacyPackaging = false
    }
}

dependencies {
    val bom = platform("androidx.compose:compose-bom:2024.12.01")
    implementation(bom)
    implementation("androidx.core:core-ktx:1.15.0")
    implementation("androidx.activity:activity-compose:1.9.3")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.8.7")
    implementation("androidx.lifecycle:lifecycle-runtime-compose:2.8.7")
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.foundation:foundation")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.material:material-icons-core")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.9.0")

    androidTestImplementation(bom)
    androidTestImplementation("androidx.compose.ui:ui-test-junit4")
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    androidTestImplementation("androidx.test:runner:1.6.2")
    androidTestImplementation("androidx.test:rules:1.6.1")
    debugImplementation("androidx.compose.ui:ui-test-manifest")
}
