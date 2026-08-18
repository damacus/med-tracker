#!/usr/bin/env ruby

require 'yaml'

contract_path = ARGV.fetch(0)
output_root = ARGV.fetch(1)
language = ARGV.fetch(2)

contract = YAML.safe_load_file(contract_path, aliases: true)
schemas = contract.fetch('components').fetch('schemas')

def pascal_case(value)
  value.split('_').map(&:capitalize).join
end

def camel_case(value)
  first, *rest = value.split('_')
  first + rest.map(&:capitalize).join
end

def clearable_fields(schema, model_name)
  clearable_fields = schema.fetch('x-client-clearable-null-fields', [])
  return clearable_fields if clearable_fields.is_a?(Array) && clearable_fields.all?(String)

  raise "Invalid clearable field marker on #{model_name}"
end

def marked_properties(schema, model_name, results)
  properties = schema.fetch('properties', {})
  clearable_fields(schema, model_name).each do |property_name|
    property = properties[property_name]
    raise "Missing clearable property #{model_name}.#{property_name}" unless property.is_a?(Hash)

    results << { model_name:, wire_name: property_name, property_name: camel_case(property_name) }
  end

  nested_objects = properties.filter do |_property_name, property|
    property.is_a?(Hash) && property['type'] == 'object'
  end
  nested_objects.each do |property_name, property|
    marked_properties(property, "#{model_name}#{pascal_case(property_name)}", results)
  end
end

marked = []
schemas.each do |schema_name, schema|
  next unless schema.is_a?(Hash)

  marked_properties(schema, schema_name, marked)
end

raise 'No clearable fields were found in the OpenAPI contract.' if marked.empty?

grouped = marked.group_by { |field| field.fetch(:model_name) }

if language == 'swift'
  models_root = File.join(output_root, 'Sources', 'MedTrackerAPI', 'Models')
  grouped.each do |model_name, fields|
    path = File.join(models_root, "#{model_name}.swift")
    source = File.read(path)
    raise "Swift marker already exists in #{model_name}" if source.include?('_explicitlyNullFields')

    fields.each do |field|
      property_name = field.fetch(:property_name)
      wire_name = field.fetch(:wire_name)
      declaration = "public var #{property_name}:"
      raise "Missing Swift property #{model_name}.#{property_name}" unless source.include?(declaration)

      encode_line = "        try container.encodeIfPresent(#{property_name}, forKey: .#{property_name})"
      replacement = [
        "        if _explicitlyNullFields.contains(\"#{wire_name}\"), #{property_name} == nil {",
        "            try container.encodeNil(forKey: .#{property_name})",
        '        } else {',
        "            try container.encodeIfPresent(#{property_name}, forKey: .#{property_name})",
        '        }'
      ].join("\n")
      raise "Missing Swift encoder line #{model_name}.#{property_name}" unless source.include?(encode_line)

      source = source.sub(encode_line, replacement)
    end

    storage = "    private var _explicitlyNullFields: Set<String> = []\n\n"
    source = source.sub('    public init(', "#{storage}    public init(")
    raise "Missing Swift initializer in #{model_name}" unless source.include?(storage)

    methods = fields.map do |field|
      property_name = field.fetch(:property_name)
      wire_name = field.fetch(:wire_name)
      [
        "    public mutating func clear#{pascal_case(wire_name)}() {",
        "        #{property_name} = nil",
        "        _explicitlyNullFields.insert(\"#{wire_name}\")",
        '    }'
      ].join("\n")
    end.join("\n\n")
    source = source.sub('    public enum CodingKeys', "#{methods}\n\n    public enum CodingKeys")
    raise "Missing Swift CodingKeys in #{model_name}" unless source.include?(methods)

    File.write(path, source)
  end
elsif language == 'kotlin'
  models_root = File.join(output_root, 'src', 'main', 'kotlin', 'io', 'medtracker', 'client', 'models')
  infrastructure_root = File.join(output_root, 'src', 'main', 'kotlin', 'io', 'medtracker', 'client', 'infrastructure')
  support_interface = <<~KOTLIN
    package io.medtracker.client.infrastructure

    interface ExplicitNullFieldsSupport {
        val explicitlyNullFields: Set<String>
    }
  KOTLIN
  support_factory = <<~KOTLIN
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
  KOTLIN

  File.write(File.join(infrastructure_root, 'ExplicitNullFieldsSupport.kt'), support_interface)
  File.write(File.join(infrastructure_root, 'ExplicitNullFieldsAdapterFactory.kt'), support_factory)

  grouped.each do |model_name, fields|
    path = File.join(models_root, "#{model_name}.kt")
    source = File.read(path)
    fields.each do |field|
      property_name = field.fetch(:property_name)
      raise "Missing Kotlin property #{model_name}.#{property_name}" unless source.include?("val #{property_name}:")
    end

    source = source.sub(
      "package io.medtracker.client.models\n",
      "package io.medtracker.client.models\n\nimport io.medtracker.client.infrastructure.ExplicitNullFieldsSupport\n"
    )
    raise "Missing Kotlin data class #{model_name}" unless source.include?("data class #{model_name} (")
    raise "Kotlin marker already exists in #{model_name}" if source.include?('explicitlyNullFields')

    closing = [
      ',',
      '',
      '    @Transient',
      '    override val explicitlyNullFields: Set<String> = emptySet()',
      '',
      ') : ExplicitNullFieldsSupport {'
    ].join("\n")
    source = source.sub("\n\n) {\n", "#{closing}\n")
    raise "Missing Kotlin model closing in #{model_name}" unless source.include?(') : ExplicitNullFieldsSupport {')

    methods = fields.map do |field|
      property_name = field.fetch(:property_name)
      wire_name = field.fetch(:wire_name)
      [
        "    fun clear#{pascal_case(wire_name)}(): #{model_name} =",
        "        copy(#{property_name} = null, explicitlyNullFields = explicitlyNullFields + \"#{wire_name}\")"
      ].join("\n")
    end.join("\n\n")
    source = source.sub(") : ExplicitNullFieldsSupport {\n", ") : ExplicitNullFieldsSupport {\n#{methods}\n")
    raise "Missing Kotlin clear methods in #{model_name}" unless source.include?(methods)

    File.write(path, source)
  end

  serializer_path = File.join(infrastructure_root, 'Serializer.kt')
  serializer = File.read(serializer_path)
  raise 'Missing KotlinJsonAdapterFactory registration' unless serializer.include?('.add(KotlinJsonAdapterFactory())')

  serializer = serializer.sub(
    '        .add(KotlinJsonAdapterFactory())',
    "        .add(ExplicitNullFieldsAdapterFactory())\n        .add(KotlinJsonAdapterFactory())"
  )
  File.write(serializer_path, serializer)

  helper_path = File.join(infrastructure_root, 'SerializerHelper.kt')
  helper = File.read(helper_path)
  marked_model_names = grouped.keys
  marked_model = false
  helper = helper.lines.map do |line|
    if line.include?('.add(io.medtracker.client.models.')
      model_name = line[/\.models\.([^.:]+)\./, 1]
      marked_model = marked_model_names.include?(model_name)
    elsif marked_model
      line = line.sub(/\)\)[ \t]*$/, ').nullSafe())') if line.include?('.withUnknownFallback(')
      marked_model = false
    end
    line
  end.join
  File.write(helper_path, helper)
else
  raise "Unknown language: #{language}"
end
