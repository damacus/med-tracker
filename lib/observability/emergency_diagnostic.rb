# frozen_string_literal: true

module Observability
  module EmergencyDiagnostic
    module_function

    def write(error:, event_name:, io: $stderr)
      io.puts(
        {
          '@timestamp' => Time.current.utc.iso8601(3),
          'log.level' => 'error',
          'event.name' => 'observability.emission_failed',
          'event.dataset' => 'medtracker.emergency',
          'error.type' => error.class.name,
          'medtracker.failed_event.name' => event_name.to_s.byteslice(0, 128)
        }.to_json
      )
    rescue StandardError
      nil
    end
  end
end
