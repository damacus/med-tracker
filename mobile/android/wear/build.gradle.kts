plugins {
    alias(libs.plugins.android.application)
}

android {
    namespace = "io.damacus.medtracker.wear"
    compileSdk { version = release(36) }
    defaultConfig {
        applicationId = "io.damacus.medtracker"
        minSdk = 30
        targetSdk = 36
        versionCode = 1
        versionName = "1.0"
    }
    buildTypes {
        debug { applicationIdSuffix = ".debug" }
        create("staging") {
            initWith(getByName("debug"))
            applicationIdSuffix = ".staging"
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

dependencies {
    implementation(project(":wear-protocol"))
    implementation(libs.wearable)
    implementation(libs.kotlinx.coroutines.android)
    implementation(libs.kotlinx.coroutines.play.services)
    testImplementation(libs.junit)
}

val companionRuntimeArtifacts = providers.provider {
    listOf("debug", "staging", "release").flatMap { variant ->
        val configuration = configurations.getByName("${variant}RuntimeClasspath")
        configuration.incoming.resolutionResult.allComponents.map { it.id.displayName }.sorted()
    }
}

abstract class VerifyCompanionDependencies : DefaultTask() {
    @get:Input
    abstract val artifacts: ListProperty<String>

    @TaskAction
    fun verify() {
        val allowed = Regex("^(project ':(wear|wear-protocol)'|com\\.google\\.android\\.gms:play-services-(wearable|base|basement|tasks):.*|com\\.google\\.guava:listenablefuture:.*|androidx\\.(annotation|collection|core|lifecycle|loader|fragment|activity|savedstate|versionedparcelable|customview|viewpager|interpolator|arch\\.core):.*|org\\.jetbrains(\\.kotlin|\\.kotlinx)?:.*)$")
        val forbidden = artifacts.get().filterNot { allowed.matches(it) }
        check(forbidden.isEmpty()) { "Wear has unapproved runtime dependencies: $forbidden" }
        println("Wear runtime dependencies contain only the protocol, Android, Google Data Layer and Kotlin libraries.")
    }
}

tasks.register<VerifyCompanionDependencies>("verifyCompanionDependencies") {
    artifacts.set(companionRuntimeArtifacts)
}
