# frozen_string_literal: true

module DatabasePoolMetricsTestSupport
  class Metric
    attr_reader :recordings

    def initialize(callback: nil)
      @callback = callback
      @recordings = []
    end

    def observe
      @callback.call
    end

    def add(value, attributes: {})
      recordings << [value, attributes]
    end
  end

  class Meter
    attr_reader :counters, :gauges

    def initialize
      @counters = {}
      @gauges = {}
    end

    def create_counter(name, **)
      counters[name] = Metric.new
    end

    def create_observable_gauge(name, callback:, **)
      gauges[name] = Metric.new(callback:)
    end
  end
end
