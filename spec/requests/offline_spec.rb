# frozen_string_literal: true

require "rails_helper"

RSpec.describe "Offline mode" do
  fixtures(
    :accounts,
    :people,
    :users,
    :locations,
    :location_memberships,
    :medications,
    :schedules,
    :person_medications,
    :medication_takes
  )

  let(:user) { users(:admin) }
  let(:medication) { medications(:gabapentin) }
  let(:schedule) do
    Schedule.create!(
      person: people(:admin),
      medication: medication,
      dose_amount: 300,
      dose_unit: "mg",
      frequency: "As needed",
      start_date: Time.zone.today,
      end_date: 1.year.from_now.to_date,
      max_daily_doses: nil,
      min_hours_between_doses: nil
    )
  end

  let(:taken_at) { 1.hour.ago.change(usec: 0) }

  before { sign_in(user) }

  describe "GET /offline" do
    it "renders the authenticated offline shell" do
      get("/offline")

      expect(response).to(have_http_status(:ok))
      expect(response.body).to(include("data-controller=\"offline-shell\""))
      expect(response.body).to(include("Offline care"))
    end
  end

  describe "GET /offline/snapshot" do
    it "returns the current care snapshot" do
      get("/offline/snapshot", as: :json)

      expect(response).to(have_http_status(:ok))
      expect(response.parsed_body.fetch("data")).to(
        include(
          "people",
          "locations",
          "medications",
          "schedules",
          "person_medications",
          "medication_takes"
        )
      )
      expect(response.parsed_body.dig("meta", "generated_at")).to(be_present)
    end
  end

  describe "POST /offline/medication_takes" do
    let(:payload) do
      {
        client_uuid: SecureRandom.uuid,
        source_type: "schedule",
        source_id: schedule.id,
        taken_at: taken_at.iso8601,
        dose_amount: "300",
        taken_from_medication_id: medication.id
      }
    end

    it "records a queued schedule take through the normal service" do
      expect do
        post("/offline/medication_takes", params: payload, as: :json)
      end
        .to(change(MedicationTake, :count).by(1))

      expect(response).to(have_http_status(:created))
      take = MedicationTake.order(:id).last
      expect(take.client_uuid).to(eq(payload.fetch(:client_uuid)))
      expect(take.schedule).to(eq(schedule))
      expect(take.taken_at).to(be_within(1.second).of(taken_at))
      expect(response.parsed_body.dig("data", "client_uuid")).to(eq(payload.fetch(:client_uuid)))
    end

    it "returns the existing take when the same client UUID is synced again" do
      post("/offline/medication_takes", params: payload, as: :json)
      created_id = response.parsed_body.dig("data", "id")

      expect do
        post("/offline/medication_takes", params: payload, as: :json)
      end
        .not_to(change(MedicationTake, :count))

      expect(response).to(have_http_status(:ok))
      expect(response.parsed_body.dig("data", "id")).to(eq(created_id))
    end

    it "returns validation errors without discarding the queued take" do
      post(
        "/offline/medication_takes",
        params: payload.merge(taken_at: 61.minutes.from_now.iso8601),
        as: :json
      )

      expect(response).to(have_http_status(:unprocessable_content))
      expect(response.parsed_body.dig("error", "message")).to(include("future"))
    end

    it "returns JSON failures for unavailable queued sources" do
      post(
        "/offline/medication_takes",
        params: payload.merge(source_id: 999_999),
        as: :json
      )

      expect(response).to(have_http_status(:unprocessable_content))
      expect(response.parsed_body.dig("error", "message")).to(include("no longer available"))
    end
  end
end
