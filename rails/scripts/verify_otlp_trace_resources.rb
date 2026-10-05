#!/usr/bin/env ruby
require 'opentelemetry/exporter/otlp'
require 'opentelemetry/proto/collector/trace/v1/trace_service_pb'
require 'zlib'

trace_files = ENV.fetch('OTLP_TRACE_FILES').split(':')
required_span_name = ENV['OTLP_REQUIRED_SPAN_NAME']
required_span_found = false
offenders = trace_files.flat_map do |path|
  payload = File.binread(path)
  payload = Zlib.gunzip(payload) if payload.start_with?("\x1F\x8B".b)
  request = Opentelemetry::Proto::Collector::Trace::V1::ExportTraceServiceRequest.decode(payload)

  if request.resource_spans.empty?
    [[path, []]]
  else
    request.resource_spans.filter_map do |resource_span|
      required_span_found ||= resource_span.scope_spans.any? do |scope_span|
        scope_span.spans.any? { |span| span.name == required_span_name }
      end if required_span_name

      attributes = resource_span.resource&.attributes || []
      host_name = attributes.find { |attribute| attribute.key == 'host.name' }&.value&.string_value
      [path, attributes.map(&:key).sort] unless host_name.is_a?(String) && !host_name.empty?
    end
  end
rescue Google::Protobuf::ParseError, Zlib::Error
  [[path, []]]
end

unless offenders.empty?
  abort "OTLP trace resource contract failed: #{offenders.map { |path, keys| "#{path} (resource keys: #{keys.join(', ')})" }.join('; ')}"
end

abort 'OTLP trace required span contract failed' if required_span_name && !required_span_found
