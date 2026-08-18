package io.medtracker.client.infrastructure

import com.squareup.moshi.JsonAdapter
import com.squareup.moshi.JsonReader
import com.squareup.moshi.JsonWriter
import com.squareup.moshi.Moshi
import com.squareup.moshi.Types
import java.lang.reflect.Type
import java.util.LinkedHashMap
import okio.Buffer

class ExplicitNullFieldsAdapterFactory : JsonAdapter.Factory {
    override fun create(
        type: Type,
        annotations: Set<Annotation>,
        moshi: Moshi
    ): JsonAdapter<*>? {
        val rawType = Types.getRawType(type)
        if (!ExplicitNullFieldsSupport::class.java.isAssignableFrom(rawType)) {
            return null
        }

        @Suppress("UNCHECKED_CAST")
        val delegate = moshi.nextAdapter<Any>(this, type, annotations)
        return object : JsonAdapter<Any>() {
            override fun fromJson(reader: JsonReader): Any? = delegate.fromJson(reader)

            override fun toJson(writer: JsonWriter, value: Any?) {
                if (value == null) {
                    writer.nullValue()
                    return
                }

                val support = value as ExplicitNullFieldsSupport
                val buffer = Buffer()
                val delegateWriter = JsonWriter.of(buffer)
                delegateWriter.serializeNulls = false
                delegate.toJson(delegateWriter, value)
                delegateWriter.close()
                val delegateValue = JsonReader.of(buffer).readJsonValue()
                val delegateObject = delegateValue as? Map<*, *>
                    ?: error("Expected an object for ${rawType.name}")
                val output = LinkedHashMap<String, Any?>(delegateObject.size + support.explicitlyNullFields.size)
                delegateObject.forEach { (key, item) -> output[key.toString()] = item }
                support.explicitlyNullFields.forEach { field ->
                    if (!output.containsKey(field)) {
                        output[field] = null
                    }
                }
                val serializeNulls = writer.serializeNulls
                writer.serializeNulls = true
                try {
                    writer.jsonValue(output)
                } finally {
                    writer.serializeNulls = serializeNulls
                }
            }
        }.nullSafe()
    }
}
