# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Observability::EventMapper do
  let(:registry) do
    YAML.safe_load_file(Rails.root.join('config/observability/signal_registry.yml'))
  end

  it 'keeps the executable event mapper aligned with the frozen event registry' do
    expect(described_class.event_names).to match_array(
      registry.fetch('custom_events').pluck('name')
    )
  end

  it 'registers every operational signal path introduced by the boundary' do
    registered_paths = registry.fetch('introduced_signal_paths').pluck('source')

    expect(registered_paths).to match_array(expected_signal_paths)
  end

  it 'gives every introduced signal path a complete disposition' do
    expect(registry.fetch('introduced_signal_paths')).to all(include(
                                                               'type',
                                                               'source',
                                                               'operational_sink',
                                                               'privacy_classification',
                                                               'failure_policy',
                                                               'owner',
                                                               'verification'
                                                             ))
  end

  it 'detects collector retries and deduplicates without dropping an application retry' do
    events = duplicate_ingestion_fixture
    duplicate_event_ids = events.group_by { |event| event.fetch('event.id') }
                                .select { |_event_id, occurrences| occurrences.many? }
                                .keys
    application_emissions = events.uniq { |event| event.fetch('event.id') }

    expect(duplicate_event_ids).to contain_exactly('43d1e299-df0d-4c7f-89d0-61706b895442')
    expect(application_emissions.size).to eq(2)
    expect(application_emissions.pluck('medtracker.attempt.id')).to contain_exactly(
      '36e156c5-35bf-4a87-a2f0-398b65f59b48',
      '715bcc5e-51e6-4471-bbe6-67d94b39db69'
    )
  end

  def duplicate_ingestion_fixture
    Rails.root.join('spec/fixtures/observability/duplicate_ingestion.ndjson')
         .each_line
         .map { |line| JSON.parse(line) }
  end

  def expected_signal_paths
    Rails.root.glob('lib/observability/*.rb').map do |path|
      path.relative_path_from(Rails.root).to_s
    end + %w[
      app/jobs/application_job.rb app/jobs/low_stock_notification_job.rb app/jobs/medication_reminder_job.rb
      app/jobs/missed_dose_notification_job.rb app/jobs/observability_canary_job.rb
      app/jobs/schedule_daily_reminders_job.rb
      app/services/medication_administration/record_dose_observability.rb
      app/services/push_notification_service.rb config/initializers/low_stock_notification_subscriber.rb
      config/initializers/observability_process_logging.rb config/initializers/observability_request_logging.rb
    ]
  end
end
