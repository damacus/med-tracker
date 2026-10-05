# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Offline mode' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :schedules,
           :person_medications, :medication_takes

  let(:user) { users(:admin) }
  let(:medication) { medications(:gabapentin) }
  let(:household) { user.person.household }
  let(:membership) do
    household.household_memberships.find_or_initialize_by(account: user.person.account).tap do |record|
      record.person = user.person
      record.role = :owner
      record.status = :active
      record.save!
    end
  end
  let(:schedule) do
    Schedule.create!(
      household: household,
      person: people(:admin),
      medication: medication,
      dose_amount: 300,
      dose_unit: 'mg',
      frequency: 'As needed',
      start_date: Date.yesterday,
      end_date: 1.year.from_now.to_date,
      max_daily_doses: nil,
      min_hours_between_doses: nil
    )
  end

  before do
    membership
    grant = household.person_access_grants.find_or_initialize_by(household_membership: membership, person: user.person)
    grant.update!(
      access_level: :manage,
      relationship_type: :self,
      granted_by_membership: membership
    )
    sign_in(user)
  end

  describe 'GET /households/:household_slug/offline' do
    it 'renders the authenticated offline shell' do
      get "/households/#{household.slug}/offline"

      expect(response).to have_http_status(:ok)
      expect(response.body).to include('data-controller="offline-shell"')
      expect(response.body).to include('Offline care')
    end

    it 'renders household-scoped endpoints and tenant cache identity' do
      get "/households/#{household.slug}/offline"

      expect(response).to have_http_status(:ok)
      expect(response.body).to include(
        "data-offline-shell-snapshot-url-value=\"/households/#{household.slug}/offline/snapshot\""
      )
      expect(response.body).to include(
        "data-offline-shell-sync-url-value=\"/households/#{household.slug}/offline/medication_takes\""
      )
      expect(response.body).to include(
        "data-offline-shell-tenant-key-value=\"household:#{household.id}:membership:#{membership.id}\""
      )
    end
  end

  describe 'GET /households/:household_slug/offline/snapshot' do
    def snapshot_selects
      queries = []
      subscriber = lambda do |*, payload|
        queries << payload[:sql] if payload[:sql].match?(/\ASELECT/i) && payload[:name] != 'SCHEMA'
      end
      ActiveSupport::Notifications.subscribed(subscriber, 'sql.active_record') do
        get "/households/#{household.slug}/offline/snapshot", as: :json
      end
      expect(response).to have_http_status(:ok)
      queries.size
    end

    it 'keeps snapshot queries bounded as schedules and direct medicines grow' do
      schedule.update!(min_hours_between_doses: 4)
      snapshot_selects
      baseline = snapshot_selects
      4.times do |index|
        stock = medication.dup
        stock.assign_attributes(name: "Query medicine #{index}", portable_id: SecureRandom.uuid)
        stock.save!
        schedule.dup.tap do |source|
          source.assign_attributes(medication: stock, portable_id: SecureRandom.uuid)
          source.save!
        end
        PersonMedication.create!(person: user.person, medication: stock, dose_amount: 300, dose_unit: 'mg',
                                 min_hours_between_doses: 4, administration_kind: :as_needed)
      end

      expect(snapshot_selects).to be <= baseline + 2
    end

    it 'includes server-assessed dose eligibility and effective dose for the offline UI' do
      schedule
      get "/households/#{household.slug}/offline/snapshot", as: :json

      source = response.parsed_body.dig('data', 'schedules').find { |item| item['id'] == schedule.id }
      expect(source.fetch('offline_eligibility')).to include(
        'allowed' => true,
        'dose_amount' => '300.0',
        'dose_unit' => 'mg'
      )
      expect(Time.iso8601(source.dig('offline_eligibility', 'valid_until'))).to be > Time.current
    end

    it 'disables inactive sources without changing snapshot visibility' do
      schedule.update!(active: false)
      get "/households/#{household.slug}/offline/snapshot", as: :json

      source = response.parsed_body.dig('data', 'schedules').find { |item| item['id'] == schedule.id }
      expect(source.fetch('offline_eligibility')).to include('allowed' => false)
      expect(source.dig('offline_eligibility', 'reason')).to be_present
    end

    it 'disables sources still in cooldown' do
      schedule.update!(min_hours_between_doses: 8)
      schedule.medication_takes.create!(taken_at: 1.hour.ago, dose_amount: 300, dose_unit: 'mg')
      get "/households/#{household.slug}/offline/snapshot", as: :json

      source = response.parsed_body.dig('data', 'schedules').find { |item| item['id'] == schedule.id }
      expect(source.fetch('offline_eligibility')).to include('allowed' => false)
    end

    it 'disables expired schedules while keeping them visible' do
      schedule.update!(start_date: 3.days.ago.to_date, end_date: 1.day.ago.to_date)
      get "/households/#{household.slug}/offline/snapshot", as: :json

      source = response.parsed_body.dig('data', 'schedules').find { |item| item['id'] == schedule.id }
      expect(source.fetch('offline_eligibility')).to include('allowed' => false)
    end

    it 'returns the current care snapshot' do
      get "/households/#{household.slug}/offline/snapshot", as: :json

      expect(response).to have_http_status(:ok)
      expect(response.parsed_body.fetch('data')).to include(
        'people',
        'locations',
        'medications',
        'schedules',
        'person_medications',
        'medication_takes'
      )
      expect(response.parsed_body.dig('meta', 'generated_at')).to be_present
    end
  end

  describe 'POST /households/:household_slug/offline/medication_takes' do
    before { travel_to(Time.current.beginning_of_day + 30.minutes) }

    def payload
      @payload ||= {
        client_uuid: SecureRandom.uuid,
        source_type: 'schedule',
        source_id: schedule.id,
        taken_at: taken_at.iso8601,
        dose_amount: '300',
        taken_from_medication_id: medication.id
      }
    end

    def taken_at
      1.hour.ago.change(usec: 0)
    end

    it 'records a queued schedule take through the normal service' do
      expect do
        post "/households/#{household.slug}/offline/medication_takes", params: payload, as: :json
      end.to change(MedicationTake, :count).by(1)

      expect(response).to have_http_status(:created)
      take = MedicationTake.order(:id).last
      expect(take.household).to eq(household)
      expect(take.client_uuid).to eq(payload.fetch(:client_uuid))
      expect(take.schedule).to eq(schedule)
      expect(take.taken_at).to be_within(1.second).of(taken_at)
      expect(response.parsed_body.dig('data', 'client_uuid')).to eq(payload.fetch(:client_uuid))
    end

    it 'returns the existing take when the same client UUID is synced again' do
      post "/households/#{household.slug}/offline/medication_takes", params: payload, as: :json
      created_id = response.parsed_body.dig('data', 'id')

      expect do
        post "/households/#{household.slug}/offline/medication_takes", params: payload, as: :json
      end.not_to change(MedicationTake, :count)

      expect(response).to have_http_status(:ok)
      expect(response.parsed_body.dig('data', 'id')).to eq(created_id)
    end

    it 'returns validation errors for future doses' do
      post "/households/#{household.slug}/offline/medication_takes",
           params: payload.merge(taken_at: 61.minutes.from_now.iso8601),
           as: :json

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.parsed_body.dig('error', 'message')).to include('future')
    end

    it 'returns JSON failures for unavailable queued sources' do
      post "/households/#{household.slug}/offline/medication_takes",
           params: payload.merge(source_id: 999_999),
           as: :json

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.parsed_body.dig('error', 'message')).to include('no longer available')
    end
  end
end
