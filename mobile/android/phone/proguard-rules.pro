# Keep Kotlin metadata and reflection annotations required by Moshi KotlinJsonAdapterFactory
-keepattributes RuntimeVisibleAnnotations, AnnotationDefault, Signature, InnerClasses, EnclosingMethod
-keep class kotlin.Metadata { *; }

# Moshi reflection & adapters
-keep class com.squareup.moshi.kotlin.reflect.KotlinJsonAdapterFactory { *; }
-keep class com.squareup.moshi.Json { *; }
-keep class com.squareup.moshi.JsonClass { *; }
-keep @com.squareup.moshi.JsonClass class * { *; }
-keepclassmembers class * {
    @com.squareup.moshi.Json <fields>;
    @com.squareup.moshi.Json <methods>;
    @com.squareup.moshi.FromJson <methods>;
    @com.squareup.moshi.ToJson <methods>;
}

# Generated OpenAPI DTOs and enums (allowshrinking preserves R8 tree-shaking of unused models)
-keep,allowshrinking class io.medtracker.client.models.** { *; }
-keepclassmembers enum io.medtracker.client.models.** {
    public static **[] values();
    public static ** valueOf(java.lang.String);
}

# Domain models serialized via kotlinx.serialization / Moshi
-keep,allowshrinking class io.damacus.medtracker.data.model.** { *; }
-keep,allowshrinking class io.damacus.medtracker.data.AppSession { *; }
-keep,allowshrinking class io.damacus.medtracker.data.AppSession$$serializer { *; }

# AppAuth (OAuth 2.0 / OIDC PKCE)
-keep class net.openid.appauth.** { *; }
-dontwarn net.openid.appauth.**

# OkHttp / Okio
-keepnames class okhttp3.internal.publicsuffix.PublicSuffixDatabase
-dontwarn okhttp3.**
-dontwarn okio.**
-dontwarn javax.annotation.**
-dontwarn org.conscrypt.**
-dontwarn org.openjsse.**
-dontwarn org.bouncycastle.**
