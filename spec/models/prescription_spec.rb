require 'rails_helper'

RSpec.describe Prescription, type: :model do
  let(:user) { User.create!(name: 'Jane Doe', email_address: 'jane@example.com', password: 'password', date_of_birth: '1990-01-01') }
  let(:medicine) { Medicine.create!(name: 'Lisinopril', current_supply: 50) }
  let(:dosage) { Dosage.create!(medicine: medicine, amount: 10, unit: 'mg', frequency: 'daily') }

  subject do
    Prescription.new(
      user: user,
      medicine: medicine,
      dosage: dosage,
      start_date: Date.today,
      end_date: Date.today + 30.days
    )
  end

  describe 'validations' do
    it { should validate_presence_of(:start_date) }
    it { should validate_presence_of(:end_date) }

    it 'is invalid if end_date is before start_date' do
      subject.end_date = subject.start_date - 1.day
      expect(subject).not_to be_valid
    end
  end

  describe 'associations' do
    it { should belong_to(:user) }
    it { should belong_to(:medicine) }
    it { should belong_to(:dosage) }
    it { should have_many(:medication_takes).dependent(:destroy) }
  end
end
