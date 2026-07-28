# frozen_string_literal: true

require 'rails_helper'
require 'erb'

module BackgroundJobConfig
end

RSpec.describe BackgroundJobConfig do
  around do |example|
    original_pool = ENV.fetch('SOLID_QUEUE_DATABASE_POOL', nil)
    ENV['SOLID_QUEUE_DATABASE_POOL'] = '8'
    example.run
  ensure
    ENV['SOLID_QUEUE_DATABASE_POOL'] = original_pool
  end

  it 'routes notification and import jobs to explicit queues' do
    notification_queues = [
      MedicationReminderJob.queue_name,
      MissedDoseNotificationJob.queue_name,
      LowStockNotificationJob.queue_name
    ]
    import_queues = [NhsDmdImportJob.queue_name, MedicationReviewEvidenceRefreshJob.queue_name]

    expect(notification_queues).to all(eq('notifications'))
    expect(import_queues).to all(eq('imports'))
  end

  it 'isolates exact queues with aggregate worker threads inside the configured queue database pool' do
    workers = queue_config.fetch('production').fetch('workers')
    queue_pool = database_config.dig('production', 'queue', 'pool')

    expect(workers.pluck('queues')).to match_array(%w[default imports notifications])
    expect(workers.sum { |worker| worker.fetch('threads') * worker.fetch('processes', 1) })
      .to be <= queue_pool - 2
  end

  it 'runs the recurring evidence refresh on the imports queue' do
    recurring = YAML.safe_load_file(Rails.root.join('config/recurring.yml'))

    expect(recurring.dig('production', 'refresh_medication_review_evidence', 'queue')).to eq('imports')
  end

  def queue_config
    YAML.safe_load(ERB.new(Rails.root.join('config/queue.yml').read).result, aliases: true)
  end

  def database_config
    YAML.safe_load(ERB.new(Rails.root.join('config/database.yml').read).result, aliases: true)
  end
end
