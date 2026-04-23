# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medication wizard dose option follow-up' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications

  before { sign_in(users(:admin)) }

  it 'replaces the wizard content with a medication-owned dose options step' do
    post medications_path,
         params: {
           wizard: 'true',
           medication: {
             name: 'Wizard Medication',
             category: 'Vitamin',
             current_supply: '10',
             reorder_threshold: '1',
             location_id: locations(:home).id,
             dosage_records_attributes: {
               '0' => {
                 amount: '2.5',
                 unit: 'ml',
                 frequency: 'Twice daily',
                 default_for_adults: '0',
                 default_for_children: '1',
                 default_max_daily_doses: '2',
                 default_min_hours_between_doses: '12',
                 default_dose_cycle: 'daily'
               }
             }
           }
         },
         headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

    expect(response).to have_http_status(:ok)
    expect(response.content_type).to include('text/vnd.turbo-stream.html')

    medication = Medication.order(:id).last
    dosage = medication.dosage_records.order(:id).last
    body = response.body

    expect(body).to include('turbo-stream')
    expect(body).to include('target="wizard-content"')
    expect(body).to include('Manage dose options')
    expect(body).to include(edit_medication_path(medication, return_to: medication_path(medication)))
    expect(body).not_to include('target="dosage-form"')
    expect(body).not_to include('target="dosage-list"')
    expect(medication.current_supply).to eq(10)
    expect(medication.reorder_threshold).to eq(1)
    expect(medication.dosage_records.count).to eq(1)
    expect(dosage).to have_attributes(
      amount: BigDecimal('2.5'),
      unit: 'ml',
      frequency: 'Twice daily',
      default_for_adults: false,
      default_for_children: true,
      default_max_daily_doses: 2,
      default_min_hours_between_doses: BigDecimal('12.0')
    )
    expect(dosage.default_dose_cycle).to eq('daily')
  end

  it 'falls back to the medication page for non-turbo requests' do
    post medications_path,
         params: {
           wizard: 'true',
           medication: {
             name: 'Redirected Wizard Medication',
             category: 'Vitamin',
             current_supply: '10',
             reorder_threshold: '1',
             location_id: locations(:home).id,
             dosage_records_attributes: {
               '0' => {
                 amount: '5',
                 unit: 'ml',
                 frequency: 'As directed',
                 default_for_adults: '1',
                 default_for_children: '0',
                 default_max_daily_doses: '1',
                 default_min_hours_between_doses: '24',
                 default_dose_cycle: 'daily'
               }
             }
           }
         }

    expect(response).to redirect_to(medication_path(Medication.order(:id).last))
  end
end
