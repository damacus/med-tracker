# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Observability::ProcessLogFormatter do
  it 'configures Puma with its structured dataset formatter' do
    expect(Rails.root.join('config/puma.rb').read).to include(
      "dataset: 'medtracker.puma'"
    )
  end

  it 'configures the Puma Solid Queue plugin from the supervisor mode environment' do
    expect(Rails.root.join('config/puma.rb').read).to include(
      "solid_queue_mode ENV.fetch('SOLID_QUEUE_SUPERVISOR_MODE', 'fork')"
    )
  end

  it 'configures Solid Queue with a warning-only structured dataset logger' do
    configuration = Rails.root.join('config/initializers/observability_process_logging.rb').read

    expect(configuration).to include(
      'SolidQueue.logger = Observability::DatasetLogger.new',
      "dataset: 'medtracker.solid_queue'",
      'level: Logger::WARN',
      'ActiveJob::Base.logger = ActiveSupport::Logger.new(IO::NULL)'
    )
  end

  it 'configures the OpenTelemetry SDK with its structured dataset logger' do
    expect(Rails.root.join('config/initializers/opentelemetry.rb').read).to include(
      'Observability::DatasetLogger.new',
      "dataset: 'medtracker.opentelemetry'"
    )
  end
end
