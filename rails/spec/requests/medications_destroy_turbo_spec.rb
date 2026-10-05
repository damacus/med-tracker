# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medications destroy with turbo_stream' do
  fixtures :accounts, :people, :users, :locations, :medications

  let(:admin) { users(:admin) }
  let(:medication) { create(:medication, household: locations(:home).household, location: locations(:home)) }

  before { sign_in(admin) }

  describe 'DELETE /medications/:id' do
    it 'returns turbo_stream and removes medication targets and updates flash' do
      delete medication_path(medication), headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(Medication.exists?(medication.id)).to be(false)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include("target=\"#{household_dom_target("medication_#{medication.id}")}\"")
      expect(response.body).to include("target=\"#{household_dom_target("medication_show_#{medication.id}")}\"")
      expect(response.body).to include('target="flash"')
    end

    it 'preserves medication history and renders a Turbo validation error when deletion is restricted' do
      historical_medication = create(:medication, household: medication.household, location: medication.location)
      historical_schedule = create(:schedule, household: medication.household, medication: historical_medication)
      create(:medication_take, :for_schedule, household: medication.household, schedule: historical_schedule)

      delete medication_path(historical_medication), headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(Medication.exists?(historical_medication.id)).to be(true)
      expect(Schedule.exists?(historical_schedule.id)).to be(true)
      expect(response.body).to include('target="flash"')
      removed_target = household_dom_target("medication_#{historical_medication.id}")
      expect(response.body).not_to include("target=\"#{removed_target}\"")
    end
  end
end
