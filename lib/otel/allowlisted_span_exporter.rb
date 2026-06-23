# frozen_string_literal: true

module Otel
  class AllowlistedSpanExporter
    SAFE_ATTRIBUTE_KEYS = Set.new(
      %w[
        db.operation
        db.system
        http.method
        http.request.method
        http.response.status_code
        http.route
        model.name
        model.operation
        network.protocol.name
        network.protocol.version
        otel.status_code
        rpc.method
        rpc.service
        server.address
        server.port
        url.scheme
      ]
    ).freeze

    def initialize(exporter)
      @exporter = exporter
    end

    def export(span_data, timeout: nil)
      exporter.export(span_data.map { |span| allowlisted_span(span) }, timeout: timeout)
    end

    def force_flush(timeout: nil)
      exporter.force_flush(timeout: timeout)
    end

    def shutdown(timeout: nil)
      exporter.shutdown(timeout: timeout)
    end

    private

    attr_reader :exporter

    def allowlisted_span(span)
      copy = span.dup
      copy.attributes = allowlisted_attributes(span.attributes || {})
      copy
    end

    def allowlisted_attributes(attributes)
      attributes.each_with_object({}) do |(key, value), allowed|
        allowed[key] = value if SAFE_ATTRIBUTE_KEYS.include?(key)
      end
    end
  end
end
