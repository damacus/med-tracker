package io.damacus.medtracker

import java.io.File
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ReleaseSecurityContractTest {
    private val androidRoot = generateSequence(File(requireNotNull(System.getProperty("user.dir")))) { it.parentFile }
        .first { File(it, "phone/build.gradle.kts").isFile }

    @Test
    fun releaseAuthenticationUsesOidcPkceWithoutPasswordOrServerOverrideUi() {
        val build = File(androidRoot, "phone/build.gradle.kts").readText()
        val libraries = File(androidRoot, "gradle/libs.versions.toml").readText()
        val releaseAuth = File(androidRoot, "phone/src/release/java/io/damacus/medtracker/AuthRoute.kt")
        val releaseGeneratedApi = File(
            androidRoot,
            "phone/src/main/kotlin/io/medtracker/client/apis/AuthenticationApi.kt"
        )
        val releaseLoginRequest = File(
            androidRoot,
            "phone/src/main/kotlin/io/medtracker/client/models/AuthLoginRequest.kt"
        )
        val nonReleaseGeneratedApi = File(
            androidRoot,
            "phone/src/nonRelease/kotlin/io/medtracker/password/client/apis/AuthenticationApi.kt"
        )
        val nonReleaseLoginRequest = File(
            androidRoot,
            "phone/src/nonRelease/kotlin/io/medtracker/client/models/AuthLoginRequest.kt"
        )
        val releaseSources = listOf(
            "MainActivity.kt",
            "data/SessionManager.kt",
            "data/api/MedTrackerApi.kt",
            "data/model/ApiModels.kt",
            "ui/MainViewModel.kt"
        ).map { File(androidRoot, "phone/src/main/java/io/damacus/medtracker/$it") } + releaseAuth
        val mergedReleaseSource = releaseSources.joinToString("\n") { it.readText() }

        assertTrue(build.contains("implementation(libs.appauth)"))
        assertTrue(libraries.contains("group = \"net.openid\", name = \"appauth\""))
        assertTrue(build.contains("create(\"staging\")"))
        assertTrue(releaseAuth.isFile)
        assertTrue(mergedReleaseSource.contains("AuthorizationRequest"))
        assertTrue(mergedReleaseSource.contains("S256"))
        assertTrue(mergedReleaseSource.contains("ResponseTypeValues.CODE"))
        assertFalse(mergedReleaseSource.contains("LoginRequest"))
        assertFalse(mergedReleaseSource.contains("Password"))
        assertFalse(mergedReleaseSource.contains("Server URL"))
        assertFalse(mergedReleaseSource.contains("updateServerUrl"))
        assertFalse(releaseGeneratedApi.readText().contains("createLoginSession"))
        assertFalse(releaseGeneratedApi.readText().contains("/auth/login"))
        assertFalse(releaseLoginRequest.exists())
        assertTrue(nonReleaseGeneratedApi.readText().contains("createLoginSession"))
        assertTrue(nonReleaseGeneratedApi.readText().contains("/auth/login"))
        assertTrue(nonReleaseLoginRequest.isFile)
        assertTrue(
            File(androidRoot, "phone/src/main/kotlin/io/medtracker/client/models/CapabilityAuthentication.kt")
                .readText().contains("password_login")
        )
    }

    @Test
    fun passwordAndServerConfigurationAreCompiledOnlyForStaging() {
        val stagingAuth = File(androidRoot, "phone/src/staging/java/io/damacus/medtracker/AuthRoute.kt")
        val stagingSource = stagingAuth.takeIf(File::isFile)?.readText().orEmpty()
        val debugSource = File(
            androidRoot,
            "phone/src/debug/java/io/damacus/medtracker/AuthRoute.kt"
        ).readText()
        val sharedPasswordUi = File(
            androidRoot,
            "phone/src/nonRelease/java/io/damacus/medtracker/auth/PasswordAuthRoute.kt"
        ).takeIf(File::isFile)?.readText().orEmpty()
        val passwordAuthenticator = File(
            androidRoot,
            "phone/src/nonRelease/java/io/damacus/medtracker/auth/PasswordAuthenticator.kt"
        ).takeIf(File::isFile)?.readText().orEmpty()
        val build = File(androidRoot, "phone/build.gradle.kts").readText()

        assertTrue(stagingSource.contains("Staging"))
        assertTrue(debugSource.contains("Debug"))
        assertTrue(stagingSource.contains("PasswordAuthRoute"))
        assertTrue(debugSource.contains("PasswordAuthRoute"))
        assertFalse(stagingSource.contains("OkHttpClient"))
        assertFalse(stagingSource.contains("AuthenticationApi"))
        assertFalse(debugSource.contains("OkHttpClient"))
        assertFalse(debugSource.contains("AuthenticationApi"))
        assertTrue(sharedPasswordUi.contains("Password"))
        assertTrue(sharedPasswordUi.contains("Server URL"))
        assertTrue(passwordAuthenticator.contains("interface PasswordAuthenticator"))
        assertTrue(passwordAuthenticator.contains("class GeneratedPasswordAuthenticator"))
        assertTrue(passwordAuthenticator.contains("HttpLoggingPolicy.client()"))
        assertTrue(passwordAuthenticator.contains("io.medtracker.password.client.apis.AuthenticationApi"))
        assertFalse(passwordAuthenticator.contains(" as "))
        assertTrue(build.contains("src/nonRelease/java"))
        assertTrue(build.contains("src/nonRelease/kotlin"))
        assertFalse(File(androidRoot, "phone/src/main/java/io/damacus/medtracker/ui/login/LoginScreen.kt").exists())
    }

    @Test
    fun credentialsUseKeystoreEncryptionAndNoBackupStorage() {
        val manifest = File(androidRoot, "phone/src/main/AndroidManifest.xml").readText()
        val sessionManager = File(
            androidRoot,
            "phone/src/main/java/io/damacus/medtracker/data/SessionManager.kt"
        ).readText()
        val credentialStore = File(
            androidRoot,
            "phone/src/main/java/io/damacus/medtracker/data/AndroidCredentialStore.kt"
        )
        val credentialSource = credentialStore.takeIf(File::isFile)?.readText().orEmpty()

        assertTrue(manifest.contains("android:allowBackup=\"false\""))
        assertFalse(manifest.contains("android:dataExtractionRules"))
        assertFalse(manifest.contains("android:fullBackupContent"))
        assertFalse(sessionManager.contains("SharedPreferences"))
        assertFalse(sessionManager.contains("getSharedPreferences"))
        assertTrue(sessionManager.contains("CredentialStore"))
        assertTrue(credentialSource.contains("noBackupFilesDir"))
        assertTrue(credentialSource.contains("AndroidKeyStore"))
        assertTrue(credentialSource.contains("AES/GCM/NoPadding"))
        assertFalse(credentialSource.contains("password", ignoreCase = true))
    }

    @Test
    fun liveCanaryTestsAreOptInAndExcludedFromOrdinaryTestsAndPullRequestCi() {
        val ordinaryCanary = File(
            androidRoot,
            "phone/src/test/java/io/damacus/medtracker/CanaryLiveIntegrationTest.kt"
        )
        val integrationCanary = File(
            androidRoot,
            "phone/src/testStagingCanaryIntegration/java/io/damacus/medtracker/CanaryLiveIntegrationTest.kt"
        )
        val integrationSource = integrationCanary.takeIf(File::isFile)?.readText().orEmpty()
        val build = File(androidRoot, "phone/build.gradle.kts").readText()
        val tasks = File(androidRoot, "Taskfile.yml").readText()
        val integrationScript = File(androidRoot, "scripts/canary-integration.fish").readText()
        val repositoryRoot = requireNotNull(androidRoot.parentFile?.parentFile)
        val workflow = File(repositoryRoot, ".github/workflows/android.yml").readText()
        val ordinaryTestTask = tasks.substringAfter("  test:").substringBefore("\n  lint:")
        val ciTask = tasks.substringAfter("  ci:")

        assertFalse(ordinaryCanary.exists())
        assertTrue(integrationCanary.isFile)
        assertTrue(build.contains("create(\"stagingCanaryIntegration\")"))
        assertTrue(build.contains("medtracker.canaryIntegration"))
        assertTrue(build.contains("testStagingCanaryIntegrationUnitTest"))
        assertTrue(build.contains("enabled = canaryIntegrationOptIn.get()"))
        assertTrue(tasks.contains("integration:canary:"))
        assertTrue(integrationScript.contains(":phone:testStagingCanaryIntegrationUnitTest"))
        assertTrue(integrationScript.contains("-Pmedtracker.canaryIntegration=true"))
        assertTrue(integrationSource.contains("MEDTRACKER_CANARY_BASE_URL"))
        assertTrue(integrationSource.contains("MEDTRACKER_CANARY_EMAIL"))
        assertTrue(integrationSource.contains("MEDTRACKER_CANARY_PASSWORD"))
        assertTrue(integrationSource.contains("GeneratedPasswordAuthenticator"))
        assertFalse(integrationSource.contains("AuthenticationApi"))
        assertFalse(integrationSource.contains("AuthLoginRequest"))
        assertFalse(integrationSource.contains("OkHttpClient"))
        assertFalse(integrationSource.contains("demo.owner@example.com"))
        assertFalse(integrationSource.contains("password = \"password\""))
        assertFalse(ordinaryTestTask.contains("StagingCanaryIntegration"))
        assertFalse(ciTask.contains("integration:canary"))
        assertFalse(workflow.contains("integration:canary"))
    }

    @Test
    fun releaseArtifactVerificationRejectsPasswordTransportDescriptors() {
        val tasks = File(androidRoot, "Taskfile.yml").readText()
        val verificationScript = File(
            androidRoot,
            "scripts/verify-release-security.fish"
        ).takeIf(File::isFile)?.readText().orEmpty()
        val ciTask = tasks.substringAfter("  ci:")

        assertTrue(tasks.contains("release:check:"))
        assertTrue(ciTask.contains("release:check"))
        assertTrue(verificationScript.contains("phone-release-unsigned.apk"))
        assertTrue(verificationScript.contains("classes*.dex"))
        assertTrue(verificationScript.contains("AuthLoginRequest"))
        assertTrue(verificationScript.contains("createLoginSession"))
        assertTrue(verificationScript.contains("/auth/login"))
        assertFalse(verificationScript.contains("password_login"))
    }

    @Test
    fun generatorMaintainsReleaseAndNonReleaseSurfacesDeterministically() {
        val update = File(androidRoot, "scripts/api-update.fish").readText()
        val check = File(androidRoot, "scripts/api-check.fish").readText()
        val manifest = File(androidRoot, "import-manifest.sha256").readText()

        assertTrue(update.contains("openapi-generator-password-config.yaml"))
        assertTrue(update.contains("openapi-generator-release.ignore"))
        assertTrue(update.contains("phone/src/nonRelease/kotlin"))
        assertTrue(check.contains("openapi-generator-password-config.yaml"))
        assertTrue(check.contains("openapi-generator-release.ignore"))
        assertTrue(check.contains("phone/src/nonRelease/kotlin"))
        assertTrue(manifest.contains("openapi-generator-password-config.yaml"))
        assertTrue(manifest.contains("openapi-generator-release.ignore"))
        assertTrue(manifest.contains("phone/src/nonRelease/kotlin"))
    }

    @Test
    fun releasePackagingRequiresExplicitValidatedBuildTimeConfiguration() {
        val build = File(androidRoot, "phone/build.gradle.kts").readText()
        val tasks = File(androidRoot, "Taskfile.yml").readText()
        val readme = File(androidRoot, "README.md").takeIf(File::isFile)?.readText().orEmpty()
        val validator = File(
            androidRoot,
            "scripts/validate-release-config.fish"
        ).takeIf(File::isFile)?.readText().orEmpty()
        val packager = File(
            androidRoot,
            "scripts/package-release.fish"
        ).takeIf(File::isFile)?.readText().orEmpty()
        val properties = listOf(
            "medtracker.release.serverUrl",
            "medtracker.release.oidcAuthorizationEndpoint",
            "medtracker.release.oidcTokenEndpoint",
            "medtracker.release.oidcClientId",
            "medtracker.release.oidcRedirectUri",
            "medtracker.release.oidcRedirectScheme"
        )

        properties.forEach {
            assertTrue(build.contains(it))
            assertTrue(readme.contains(it))
            assertTrue(packager.contains(it))
        }
        assertTrue(build.contains("invalid.medtracker.example"))
        assertTrue(tasks.contains("release:validate:"))
        assertTrue(tasks.contains("package:release:"))
        assertTrue(validator.contains("invalid"))
        assertTrue(packager.contains(":phone:assembleRelease"))
        assertTrue(packager.contains("verify-release-security.fish"))
        assertTrue(readme.contains("task android:package:release"))
        assertTrue(readme.contains("password_login"))
        assertTrue(readme.contains("capability"))
    }
}
