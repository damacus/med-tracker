# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medications edit return_to sanitization' do
  fixtures :accounts, :people, :users, :locations, :medications

  let(:admin) { users(:admin) }
  let(:medication) { medications(:paracetamol) }

  before { sign_in(admin) }

  describe 'GET /medications/:id/edit' do
    it 'shows an editable friendly display name field' do
      medication = Medication.create!(
        name: 'Paracetamol 500mg tablets',
        friendly_name: 'Short Paracetamol',
        location: locations(:home),
        reorder_threshold: 5
      )

      get edit_medication_path(medication)

      expect(response.body).to include('Display name')
      expect(response.body).to include('name="medication[friendly_name]"')
      expect(response.body).to include('value="Short Paracetamol"')
    end

    it 'preserves a safe internal return_to path' do
      get edit_medication_path(medication, return_to: '/medications')
      expect(response.body).to include('href="/medications"')
    end

    it 'renders a new dosage row without editing the existing dosage when adding a dosage' do
      medication = Medication.create!(
        name: 'Multi Dose Medication',
        location: locations(:home),
        reorder_threshold: 5
      )
      medication.dosage_records.create!(
        amount: 2,
        unit: 'sachet',
        frequency: 'Once daily',
        default_for_children: true,
        default_max_daily_doses: 1,
        default_min_hours_between_doses: 24,
        default_dose_cycle: :daily
      )

      get edit_medication_path(medication, add_dosage: true)

      expect(response.body).to include('name="medication[dosage_records_attributes][0][id]"')
      expect(response.body).to include('name="medication[dosage_records_attributes][1][amount]"')
      expect(response.body).not_to include('name="medication[dosage_records_attributes][1][id]"')
    end

    it 'strips an external return_to url from rendered links' do
      get edit_medication_path(medication, return_to: 'https://evil.com/phish')
      expect(response.body).not_to include('evil.com')
    end

    it 'strips a javascript: return_to scheme from rendered links' do
      get edit_medication_path(medication, return_to: 'javascript:alert(1)')
      expect(response.body).not_to include('javascript:alert')
    end
  end
end
