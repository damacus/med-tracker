require 'rails_helper'

RSpec.describe Medicine, type: :model do
  subject { Medicine.new(name: 'Ibuprofen', current_supply: 200) }

  describe 'validations' do
    it { should validate_presence_of(:name) }
    it { should validate_presence_of(:current_supply) }
    it { should validate_numericality_of(:current_supply).only_integer.is_greater_than_or_equal_to(0) }
  end

  describe 'associations' do
    it { should have_many(:dosages).dependent(:destroy) }
    it { should have_many(:prescriptions).dependent(:destroy) }
  end
end
