# frozen_string_literal: true

module Observability
  module CanonicalLogger
    LOCK = Mutex.new
    NULL_OUTPUT = File.open(IO::NULL, 'w')

    module_function

    def write(event, io: default_output)
      LOCK.synchronize { io.puts(event.to_json) }
    end

    def default_output
      Rails.env.test? ? NULL_OUTPUT : $stdout
    end
    private_class_method :default_output
  end
end
