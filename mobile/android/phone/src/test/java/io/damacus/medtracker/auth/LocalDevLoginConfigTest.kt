package io.damacus.medtracker.auth

import java.io.File
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class LocalDevLoginConfigTest {
    private val androidRoot = generateSequence(File(requireNotNull(System.getProperty("user.dir")))) { it.parentFile }
        .first { File(it, "phone/build.gradle.kts").isFile }

    @Test
    fun mainNetworkSecurityConfigDisallowsAllCleartextAndDebugAllowsLocalOnly() {
        val mainConfig = File(androidRoot, "phone/src/main/res/xml/network_security_config.xml").readText()
        val debugConfigFile = File(androidRoot, "phone/src/debug/res/xml/network_security_config.xml")

        assertTrue(mainConfig.contains("cleartextTrafficPermitted=\"false\""))
        assertFalse("Main network_security_config must not allow cleartext traffic", mainConfig.contains("cleartextTrafficPermitted=\"true\""))
        assertFalse("Main network_security_config must not include 10.0.2.2", mainConfig.contains("10.0.2.2"))
        assertFalse("Main network_security_config must not include localhost", mainConfig.contains("localhost"))
        assertFalse("Main network_security_config must not include 127.0.0.1", mainConfig.contains("127.0.0.1"))

        assertTrue("Debug network_security_config must exist", debugConfigFile.isFile)
        val debugConfig = debugConfigFile.readText()
        assertTrue(debugConfig.contains("<base-config cleartextTrafficPermitted=\"true\""))
    }

    @Test
    fun buildGradleSetsDebugLocalServerUrlWhileKeepingStagingAndReleaseHttps() {
        val build = File(androidRoot, "phone/build.gradle.kts").readText()
        val debugBlock = build.substringAfter("getByName(\"debug\")").substringBefore("create(\"staging\")")
        val stagingBlock = build.substringAfter("create(\"staging\")").substringBefore("release {")
        val releaseBlock = build.substringAfter("release {")

        assertTrue(
            "Debug build type should default SERVER_URL to http://10.0.2.2:5150/",
            debugBlock.contains("http://10.0.2.2:5150/")
        )
        assertTrue(
            "Staging build type must remain HTTPS",
            stagingBlock.contains("https://invalid-staging.medtracker.example/")
        )
        assertFalse(
            "Release build type must not hardcode http://",
            releaseBlock.contains("http://")
        )
    }

    @Test
    fun mainActivityEnablesLocalHttpOnlyInDebugAndWiresBackHandlerAndActiveRefresh() {
        val mainActivity = File(
            androidRoot,
            "phone/src/main/java/io/damacus/medtracker/MainActivity.kt"
        ).readText()

        assertTrue(
            "MobileAuthDiscovery must pass allowLocalHttp = BuildConfig.DEBUG",
            mainActivity.contains("MobileAuthDiscovery(allowLocalHttp = BuildConfig.DEBUG)")
        )
        assertTrue(
            "MainActivity must use AuthorizationRequest.Builder.setResponseMode(AuthorizationRequest.ResponseMode.QUERY)",
            mainActivity.contains(".setResponseMode(AuthorizationRequest.ResponseMode.QUERY)")
        )
        assertFalse(
            "MainActivity must not pass response_mode via setAdditionalParameters",
            mainActivity.contains("setAdditionalParameters(mapOf(\"response_mode\"")
        )
        assertTrue(
            "MainActivity must configure AuthorizationService with LocalDevConnectionBuilder",
            mainActivity.contains("LocalDevConnectionBuilder.appAuthConfiguration(BuildConfig.DEBUG)")
        )
        val mobileSessionAuth = File(
            androidRoot,
            "phone/src/main/java/io/damacus/medtracker/auth/MobileSessionAuthentication.kt"
        ).readText()
        assertTrue(
            "MobileSessionAuthentication must configure AuthorizationService with LocalDevConnectionBuilder",
            mobileSessionAuth.contains("LocalDevConnectionBuilder.appAuthConfiguration(BuildConfig.DEBUG)")
        )
        assertTrue(
            "MainActivity must register BackHandler for overlays",
            mainActivity.contains("BackHandler(")
        )
        assertTrue(
            "Top bar refresh must dispatch based on currentDestination",
            mainActivity.contains("when (currentDestination)")
        )
    }

    @Test
    fun proguardRulesKeepMoshiOpenApiModelsAppAuthAndOkHttpWithoutKeepingAuthLoginRequest() {
        val proguard = File(androidRoot, "phone/proguard-rules.pro").readText()

        assertTrue("Proguard rules must keep Moshi KotlinJsonAdapterFactory", proguard.contains("KotlinJsonAdapterFactory"))
        assertTrue("Proguard rules must keep Moshi Json annotation", proguard.contains("com.squareup.moshi.Json"))
        assertTrue("Proguard rules must keep generated OpenAPI model members", proguard.contains("io.medtracker.client.models."))
        assertTrue("Proguard rules must keep AppAuth classes", proguard.contains("net.openid.appauth."))
        assertTrue("Proguard rules must keep OkHttp rules", proguard.contains("okhttp3."))
        assertFalse(
            "Proguard rules must not force-keep forbidden AuthLoginRequest",
            proguard.contains("AuthLoginRequest")
        )
    }
}
