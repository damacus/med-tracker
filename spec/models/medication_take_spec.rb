require 'rails_helper'

RSpec.describe MedicationTake, type: :model do
  let(:user) { User.create!(name: 'Jane Doe', email_address: 'jane@example.com', password: 'password', date_of_birth: '1990-01-01') }
  let(:medicine) { Medicine.create!(name: 'Lisinopril', current_supply: 50) }
  let(:dosage) { Dosage.create!(medicine: medicine, amount: 10, unit: 'mg', frequency: 'daily') }
  let(:prescription) do
    Prescription.create!(
      user: user,
      medicine: medicine,
      dosage: dosage,
      start_date: Date.today,
      end_date: Date.today + 30.days
    )
  end

  subject { MedicationTake.new(prescription: prescription, taken_at: Time.current) }

  describe 'validations' do
    it { should validate_presence_of(:taken_at) }
  end

  describe 'associations' do
    it { should belong_to(:prescription) }
  end
end
