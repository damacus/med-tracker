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

    def record(value, attributes: {})
      recordings << [value, attributes]
    end
  end

  class Meter
    attr_reader :counters, :gauges, :observable_gauges

    def initialize
      @counters = {}
      @gauges = {}
      @observable_gauges = {}
    end

    def create_counter(name, **)
      counters[name] = Metric.new
    end

    def create_observable_gauge(name, callback:, **)
      observable_gauges[name] = Metric.new(callback:)
    end

    def create_gauge(name, **)
      gauges[name] = Metric.new
    end

    def collect
      observable_gauges.each_value(&:observe)
    end
  end
end
