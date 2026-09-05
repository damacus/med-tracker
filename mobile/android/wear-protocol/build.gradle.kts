plugins {
    alias(libs.plugins.kotlin.jvm)
}

kotlin { jvmToolchain(17) }

dependencies {
    implementation(libs.kotlinx.serialization.json)
    testImplementation(libs.junit)
}
