require 'rails_helper'

RSpec.describe Dosage, type: :model do
  let(:medicine) { Medicine.create!(name: 'Aspirin', current_supply: 100) }
  subject { Dosage.new(medicine: medicine, amount: 500, unit: 'mg', frequency: 'daily', description: 'Take with water') }

  describe 'validations' do
    it { should validate_presence_of(:amount) }
    it { should validate_presence_of(:unit) }
    it { should validate_presence_of(:frequency) }
    it { should validate_numericality_of(:amount).is_greater_than(0) }
  end

  describe 'associations' do
    it { should belong_to(:medicine) }
    it { should have_many(:prescriptions).dependent(:destroy) }
  end
end
