# frozen_string_literal: true

module Otel
  class AllowlistedSpanExporter
    SAFE_ATTRIBUTE_KEYS = Set.new(
      %w[
        db.operation
        db.system
        error.type
        exception.escaped
        exception.source
        http.method
        http.request.method
        http.response.status_code
        http.route
        model.id_hash
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
    SAFE_RESOURCE_KEYS = Set.new(
      %w[
        service.name
        service.namespace
        service.version
        host.name
        deployment.environment
        telemetry.sdk.language
        telemetry.sdk.name
        telemetry.sdk.version
      ]
    ).freeze
    SAFE_SPAN_NAME = /\A(?:[a-z][a-z0-9_.:-]{0,127}|[A-Z][A-Za-z0-9_:]{0,127}Job)\z/

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
      sanitize_identity(copy, span)
      sanitize_optional_surface(copy, span)
      copy
    end

    def sanitize_identity(copy, span)
      copy.name = allowlisted_name(span.name) if copy.respond_to?(:name=)
      copy.attributes = allowlisted_attributes(span.attributes || {})
      copy.events = [] if copy.respond_to?(:events=)
    end

    def sanitize_optional_surface(copy, span)
      sanitize_status(copy, span)
      sanitize_links(copy, span)
      sanitize_resource(copy, span)
    end

    def sanitize_status(copy, span)
      copy.status = allowlisted_status(span.status) if copy.respond_to?(:status=) && span.respond_to?(:status)
    end

    def sanitize_links(copy, span)
      copy.links = allowlisted_links(span.links) if copy.respond_to?(:links=) && span.respond_to?(:links)
    end

    def sanitize_resource(copy, span)
      copy.resource = allowlisted_resource(span.resource) if copy.respond_to?(:resource=) && span.respond_to?(:resource)
    end

    def allowlisted_name(name)
      value = name.to_s
      value.match?(SAFE_SPAN_NAME) ? value : 'span.filtered'
    end

    def allowlisted_attributes(attributes)
      attributes.each_with_object({}) do |(key, value), allowed|
        allowed[key] = value if SAFE_ATTRIBUTE_KEYS.include?(key)
      end
    end

    def allowlisted_status(status)
      return status unless status.respond_to?(:description)

      if status.is_a?(OpenTelemetry::Trace::Status)
        return OpenTelemetry::Trace::Status.ok if status.code == OpenTelemetry::Trace::Status::OK
        return OpenTelemetry::Trace::Status.error if status.code == OpenTelemetry::Trace::Status::ERROR

        return OpenTelemetry::Trace::Status.unset
      end

      copy = status.dup
      copy.description = nil
      copy
    end

    def allowlisted_links(links)
      Array(links).map do |link|
        next OpenTelemetry::Trace::Link.new(link.span_context, {}) if link.is_a?(OpenTelemetry::Trace::Link)

        copy = link.dup
        copy.attributes = {} if copy.respond_to?(:attributes=)
        copy
      end
    end

    def allowlisted_resource(resource)
      if resource.respond_to?(:attribute_enumerator)
        attributes = resource.attribute_enumerator.to_h.slice(*SAFE_RESOURCE_KEYS)
        return OpenTelemetry::SDK::Resources::Resource.create(attributes)
      end
      return resource unless resource.respond_to?(:attributes)

      copy = resource.dup
      copy.attributes = resource.attributes.slice(*SAFE_RESOURCE_KEYS)
      copy
    end
  end
end
