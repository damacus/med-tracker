# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'Medication dose option updates' do
  fixtures :accounts, :people, :users, :locations, :location_memberships

  before { sign_in(users(:admin)) }

  it 'replaces the existing child default when a newly added fractional dose is submitted as the child default' do
    medication = Medication.create!(
      name: 'Wellbaby Multivitamin drops (Vitabiotics Ltd) 30 ml',
      friendly_name: 'Wellbaby Multivitamin drops',
      category: 'Vitamin',
      location: locations(:home),
      current_supply: 21,
      reorder_threshold: 0
    )
    existing_dosage = medication.dosage_records.create!(
      amount: 1.0,
      unit: 'ml',
      frequency: 'Once daily',
      default_for_children: true,
      default_max_daily_doses: 1,
      default_min_hours_between_doses: 24,
      default_dose_cycle: :daily
    )

    expect do
      patch medication_path(medication),
            params: {
              return_to: medication_path(medication),
              medication: {
                name: medication.name,
                friendly_name: medication.friendly_name,
                category: medication.category,
                location_id: medication.location_id,
                current_supply: '21',
                reorder_threshold: '0',
                dosage_records_attributes: {
                  '0' => {
                    id: existing_dosage.id,
                    amount: '1.0',
                    unit: 'ml',
                    frequency: 'Once daily',
                    description: '',
                    default_for_adults: '0',
                    default_for_children: '1',
                    default_max_daily_doses: '1',
                    default_min_hours_between_doses: '24',
                    default_dose_cycle: 'daily',
                    current_supply: '',
                    reorder_threshold: '',
                    _destroy: '0'
                  },
                  '1' => {
                    amount: '0.5',
                    unit: 'ml',
                    frequency: 'Twice daily',
                    description: '',
                    default_for_adults: '0',
                    default_for_children: '1',
                    default_max_daily_doses: '2',
                    default_min_hours_between_doses: '12',
                    default_dose_cycle: 'daily',
                    current_supply: '',
                    reorder_threshold: '',
                    _destroy: '0'
                  }
                }
              }
            }
    end.to change { medication.dosage_records.count }.by(1)

    expect(response).to redirect_to(medication_path(medication))
    expect(existing_dosage.reload.default_for_children).to be(false)
    expect(medication.dosage_records.find_by!(amount: BigDecimal('0.5'), unit: 'ml')).to be_default_for_children
  end
end
