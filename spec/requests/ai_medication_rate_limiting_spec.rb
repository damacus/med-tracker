# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'AI medication suggestion rate limiting' do
  include ActiveSupport::Testing::TimeHelpers

  fixtures :accounts, :people, :users, :households, :locations, :location_memberships

  let(:admin) { users(:admin) }
  let(:household) { Household.find_by!(slug: default_request_household_slug) }
  let(:suggestion) { AiMedication::Suggestion.new(medication: { description: 'Draft' }) }
  let(:service) { instance_double(AiMedication::SuggestionService, call: suggestion) }

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
    household.update!(subscription_plan: 'family_plus')
    allow(ENV).to receive(:fetch).with('MEDTRACKER_AI_MEDICATION_HELP_ENABLED', 'false').and_return('true')
    allow(AiMedication::SuggestionService).to receive(:new).and_return(service)
  end

  it 'throttles tenant-scoped AI suggestions by IP' do
    notifications = []
    subscriber = lambda do |_name, _started, _finished, _id, payload|
      notifications << payload
    end

    ActiveSupport::Notifications.subscribed(subscriber, 'rack_attack.throttled') do
      10.times { post ai_medication_suggestions_path, params: suggestion_params }
      expect(response).to have_http_status(:ok)

      post ai_medication_suggestions_path, params: suggestion_params
    end

    expect(response).to have_http_status(:too_many_requests)
    expect(response.headers['Retry-After'].to_i).to be_positive
    expect(notifications).to include(hash_including(throttle: 'ai_medication_suggestions/ip'))
  end

  it 'throttles tenant-scoped AI suggestions by signed-in account' do
    20.times do |index|
      post ai_medication_suggestions_path,
           params: suggestion_params,
           headers: { 'REMOTE_ADDR' => "203.0.113.#{index}" }
    end
    expect(response).to have_http_status(:ok)

    post ai_medication_suggestions_path,
         params: suggestion_params,
         headers: { 'REMOTE_ADDR' => '203.0.113.250' }

    expect(response).to have_http_status(:too_many_requests)
    expect(response.headers['Retry-After'].to_i).to be_positive
  end

  def suggestion_params
    { medication: { name: 'Calpol Six Plus' } }
  end
end
