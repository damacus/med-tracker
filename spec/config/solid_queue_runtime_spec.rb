# frozen_string_literal: true

require 'rails_helper'
require 'erb'

module SolidQueueRuntime
end

RSpec.describe SolidQueueRuntime do
  it 'uses fiber-scoped Rails execution state' do
    expect(Rails.application.config.active_support.isolation_level).to eq(:fiber)
  end

  it 'runs notification jobs on a bounded fiber worker' do
    notification_worker = workers.find { |worker| worker.fetch('queues') == 'notifications' }

    expect(notification_worker).to include('fibers' => 2, 'polling_interval' => 0.1)
    expect(notification_worker).not_to have_key('threads')
  end

  it 'keeps blocking jobs on one thread' do
    default_worker = workers.find { |worker| worker.fetch('queues') == 'default' }

    expect(default_worker).to include('threads' => 1, 'polling_interval' => 0.1)
    expect(default_worker).not_to have_key('fibers')
  end

  it 'runs the Puma-hosted supervisor asynchronously' do
    expect(puma_config).to include("solid_queue_mode ENV.fetch('SOLID_QUEUE_SUPERVISOR_MODE', 'fork')")
    expect(deploy_config.dig('env', 'clear', 'SOLID_QUEUE_SUPERVISOR_MODE')).to eq('async')
  end

  it 'includes the Async runtime dependency' do
    expect(gemfile).to include("gem 'async', '>= 2.24'")
  end

  it 'routes notification delivery jobs to the fiber worker' do
    expect(notification_jobs.map(&:queue_name)).to all(eq('notifications'))
  end

  it 'sizes the shared primary and queue connection pools explicitly' do
    expect(database_config.dig('development', 'primary', 'pool')).to eq(8)
    expect(database_config.dig('development', 'queue', 'pool')).to eq(8)
    expect(database_config.dig('production', 'primary', 'pool')).to eq(8)
    expect(database_config.dig('production', 'queue', 'pool')).to eq(8)
  end

  it 'passes Solid Queue runtime validation in async mode' do
    configuration = SolidQueue::Configuration.new(mode: :async, skip_recurring: true)

    expect(configuration).to be_valid
  end

  def workers
    queue_config.fetch('production').fetch('workers')
  end

  def queue_config
    YAML.safe_load(ERB.new(Rails.root.join('config/queue.yml').read).result, aliases: true)
  end

  def database_config
    YAML.safe_load(ERB.new(Rails.root.join('config/database.yml').read).result, aliases: true)
  end

  def puma_config
    Rails.root.join('config/puma.rb').read
  end

  def deploy_config
    YAML.safe_load(Rails.root.join('config/deploy.yml').read, aliases: true)
  end

  def gemfile
    Rails.root.join('Gemfile').read
  end

  def notification_jobs
    [MedicationReminderJob, MissedDoseNotificationJob, LowStockNotificationJob]
  end
end
