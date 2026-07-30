# frozen_string_literal: true

module Observability
  class DatasetLogger < Logger
    def initialize(io = $stdout, dataset:, level: Logger::INFO)
      super(io, level:)
      self.formatter = lambda do |severity, time, _program_name, message|
        "#{ProcessLogFormatter.call(message, dataset:, severity:, time:)}\n"
      end
    end
  end
end
