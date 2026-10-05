# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medication lookup rate limiting' do
  include ActiveSupport::Testing::TimeHelpers

  fixtures :accounts, :people, :users

  let(:admin) { users(:admin) }

  around do |example|
    original_cache_store = Rack::Attack.cache.store
    original_enabled = Rack::Attack.enabled

    Rack::Attack.cache.store = ActiveSupport::Cache::MemoryStore.new
    Rack::Attack.enabled = true

    example.run
  ensure
    Rack::Attack.cache.store = original_cache_store
    Rack::Attack.enabled = original_enabled
  end

  before do
    freeze_time
    sign_in(admin)
    stub_successful_lookup
  end

  it 'allows ordinary lookup traffic below the threshold' do
    3.times { get medication_finder_search_path(format: :json), params: { q: 'aspirin' } }

    expect(response).to have_http_status(:ok)
  end

  it 'throttles excessive lookup traffic and emits retry metadata' do
    notifications = []
    subscriber = lambda do |_name, _started, _finished, _id, payload|
      notifications << payload
    end

    ActiveSupport::Notifications.subscribed(subscriber, 'rack_attack.throttled') do
      60.times { get medication_finder_search_path(format: :json), params: { q: 'aspirin' } }
      expect(response).to have_http_status(:ok)

      get medication_finder_search_path(format: :json), params: { q: 'aspirin' }
    end

    expect(response).to have_http_status(:too_many_requests)
    expect(response.body).to include('Rate limit exceeded')
    expect(response.headers['Retry-After'].to_i).to be > 0
    expect(notifications).to include(hash_including(throttle: 'medication_lookup/ip'))
  end

  def stub_successful_lookup
    search = instance_double(
      NhsDmd::Search,
      call: NhsDmd::Search::Result.new(results: [], error: nil)
    )
    allow(NhsDmd::Search).to receive(:new).and_return(search)
  end
end
