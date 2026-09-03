import com.android.build.api.variant.ApplicationVariantBuilder
import com.android.build.api.variant.HostTestBuilder
import org.gradle.api.tasks.testing.Test

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.compose.compiler)
    alias(libs.plugins.kotlin.serialization)
}

val httpLoggingOptIn = providers.gradleProperty("medtracker.httpLogging")
    .map(String::toBoolean)
    .orElse(false)
val canaryIntegrationOptIn = providers.gradleProperty("medtracker.canaryIntegration")
    .map(String::toBoolean)
    .orElse(false)
val releaseServerUrl = providers.gradleProperty("medtracker.release.serverUrl")
    .orElse("https://invalid.medtracker.example/")
val releaseOidcAuthorizationEndpoint = providers.gradleProperty("medtracker.release.oidcAuthorizationEndpoint")
    .orElse("https://invalid-id.medtracker.example/authorize")
val releaseOidcTokenEndpoint = providers.gradleProperty("medtracker.release.oidcTokenEndpoint")
    .orElse("https://invalid-id.medtracker.example/token")
val releaseOidcClientId = providers.gradleProperty("medtracker.release.oidcClientId")
    .orElse("invalid-medtracker-android-release")
val releaseOidcRedirectUri = providers.gradleProperty("medtracker.release.oidcRedirectUri")
    .orElse("io.damacus.medtracker.invalid:/oauth2redirect")
val releaseOidcRedirectScheme = providers.gradleProperty("medtracker.release.oidcRedirectScheme")
    .orElse("io.damacus.medtracker.invalid")

fun String.asBuildConfigString(): String =
    "\"${replace("\\", "\\\\").replace("\"", "\\\"")}\""

android {
    namespace = "io.damacus.medtracker"
    compileSdk {
        version = release(36)
    }

    defaultConfig {
        applicationId = "io.damacus.medtracker"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "1.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildTypes {
        getByName("debug") {
            applicationIdSuffix = ".debug"
            buildConfigField("String", "SERVER_URL", "\"https://invalid-debug.medtracker.example/\"")
            buildConfigField("String", "OIDC_AUTHORIZATION_ENDPOINT", "\"https://invalid-id.medtracker.example/authorize\"")
            buildConfigField("String", "OIDC_TOKEN_ENDPOINT", "\"https://invalid-id.medtracker.example/token\"")
            buildConfigField("String", "OIDC_CLIENT_ID", "\"invalid-medtracker-android-debug\"")
            buildConfigField("String", "OIDC_REDIRECT_URI", "\"io.damacus.medtracker.debug:/oauth2redirect\"")
            buildConfigField("boolean", "IS_RELEASE_BUILD", "false")
            buildConfigField("boolean", "HTTP_LOGGING_OPT_IN", httpLoggingOptIn.get().toString())
            manifestPlaceholders["appAuthRedirectScheme"] = "io.damacus.medtracker.debug"
        }
        create("staging") {
            initWith(getByName("debug"))
            applicationIdSuffix = ".staging"
            buildConfigField("String", "SERVER_URL", "\"https://invalid-staging.medtracker.example/\"")
            buildConfigField("String", "OIDC_REDIRECT_URI", "\"io.damacus.medtracker.staging:/oauth2redirect\"")
            buildConfigField("boolean", "IS_RELEASE_BUILD", "false")
            buildConfigField("boolean", "HTTP_LOGGING_OPT_IN", httpLoggingOptIn.get().toString())
            manifestPlaceholders["appAuthRedirectScheme"] = "io.damacus.medtracker.staging"
            matchingFallbacks += listOf("debug")
        }
        create("stagingCanaryIntegration") {
            initWith(getByName("staging"))
            applicationIdSuffix = ".canary.integration"
            matchingFallbacks += listOf("staging", "debug")
        }
        release {
            isMinifyEnabled = true
            buildConfigField("String", "SERVER_URL", releaseServerUrl.get().asBuildConfigString())
            buildConfigField(
                "String",
                "OIDC_AUTHORIZATION_ENDPOINT",
                releaseOidcAuthorizationEndpoint.get().asBuildConfigString()
            )
            buildConfigField("String", "OIDC_TOKEN_ENDPOINT", releaseOidcTokenEndpoint.get().asBuildConfigString())
            buildConfigField("String", "OIDC_CLIENT_ID", releaseOidcClientId.get().asBuildConfigString())
            buildConfigField("String", "OIDC_REDIRECT_URI", releaseOidcRedirectUri.get().asBuildConfigString())
            buildConfigField("boolean", "IS_RELEASE_BUILD", "true")
            buildConfigField("boolean", "HTTP_LOGGING_OPT_IN", "false")
            manifestPlaceholders["appAuthRedirectScheme"] = releaseOidcRedirectScheme.get()
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    buildFeatures {
        compose = true
        buildConfig = true
    }
    sourceSets {
        listOf("debug", "staging", "stagingCanaryIntegration").forEach { buildType ->
            named(buildType) {
                kotlin.directories.addAll(listOf("src/nonRelease/java", "src/nonRelease/kotlin"))
            }
        }
    }
}

androidComponents {
    beforeVariants(selector().withBuildType("stagingCanaryIntegration")) { variant: ApplicationVariantBuilder ->
        variant.hostTests.getValue(HostTestBuilder.UNIT_TEST_TYPE).enable = true
    }
}

tasks.withType<Test>().configureEach {
    if (name == "testStagingCanaryIntegrationUnitTest") {
        enabled = canaryIntegrationOptIn.get()
    }
}

dependencies {
    implementation(project(":wear-protocol"))
    implementation(libs.wearable)
    implementation(libs.kotlinx.coroutines.play.services)
    val composeBom = platform(libs.androidx.compose.bom)
    implementation(composeBom)
    androidTestImplementation(composeBom)

    implementation(libs.androidx.compose.ui)
    implementation(libs.androidx.compose.ui.graphics)
    implementation(libs.androidx.compose.ui.tooling.preview)
    implementation(libs.androidx.compose.material3)
    implementation(libs.androidx.compose.material.icons.extended)
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.lifecycle.viewmodel.compose)
    implementation(libs.androidx.lifecycle.runtime.compose)

    debugImplementation(libs.androidx.compose.ui.tooling)
    debugImplementation(libs.androidx.compose.ui.test.manifest)

    implementation(libs.okhttp)
    implementation(libs.okhttp.logging)
    implementation(libs.moshi)
    implementation(libs.moshi.kotlin)
    implementation(libs.moshi.adapters)
    implementation(libs.kotlinx.serialization.json)
    implementation(libs.kotlinx.coroutines.android)
    implementation(libs.appauth)

    implementation(libs.androidx.core.ktx)
    implementation(libs.androidx.appcompat)
    implementation(libs.material)

    testImplementation(libs.junit)
    testImplementation(libs.okhttp.mockwebserver)
    androidTestImplementation(libs.androidx.junit)
    androidTestImplementation(libs.androidx.espresso.core)
    androidTestImplementation(libs.androidx.compose.ui.test.junit4)
}
