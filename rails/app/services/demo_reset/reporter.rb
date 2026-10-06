# frozen_string_literal: true

module DemoReset
  class Reporter
    def initialize(output: $stdout)
      @output = output
    end

    def stage(name, outcome)
      output.puts("demo_reset stage=#{name} outcome=#{outcome}")
    end

    private

    attr_reader :output
  end
end
