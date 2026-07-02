# frozen_string_literal: true

require 'opentelemetry/sdk/trace/samplers'

module Otel
  class CriticalPathSampler
    ATTRIBUTE_KEYS = %w[
      http.route
      http.target
      url.path
    ].freeze

    def initialize(delegate:, critical_path_matchers:)
      @delegate = delegate
      @critical_path_matchers = critical_path_matchers
    end

    def should_sample?(**sampling_options)
      name = sampling_options[:name]
      attributes = sampling_options[:attributes]

      return always_on.should_sample?(**sampling_options) if critical_path?(name, attributes || {})

      delegate.should_sample?(**sampling_options)
    end

    def description
      "CriticalPathSampler{matchers=#{critical_path_matchers.size}, delegate=#{delegate.description}}"
    end

    private

    attr_reader :delegate, :critical_path_matchers

    def always_on
      OpenTelemetry::SDK::Trace::Samplers::ALWAYS_ON
    end

    def critical_path?(name, attributes)
      candidates = [name, *ATTRIBUTE_KEYS.filter_map { |key| attributes[key] }]
      candidates.any? { |candidate| critical_path_matchers.any? { |matcher| matches?(matcher, candidate) } }
    end

    def matches?(matcher, candidate)
      matcher.is_a?(Regexp) ? matcher.match?(candidate.to_s) : candidate.to_s.include?(matcher.to_s)
    end
  end
end
