require 'rails_helper'

RSpec.describe User, type: :model do
  subject { User.new(name: 'John Doe', email_address: 'test@example.com', password: 'password', password_confirmation: 'password', date_of_birth: '2000-01-01') }

  describe 'validations' do
    it { should validate_presence_of(:name) }
    it { should validate_presence_of(:email_address) }
    it { should validate_uniqueness_of(:email_address).case_insensitive }
    it { should allow_value('user@example.com').for(:email_address) }
    it { should_not allow_value('user@example').for(:email_address) }
    it { should_not allow_value('userexample.com').for(:email_address) }
    it { should validate_presence_of(:date_of_birth) }
  end

  describe 'associations' do
    it { should have_many(:sessions).dependent(:destroy) }
    it { should have_many(:prescriptions).dependent(:destroy) }
  end

  describe 'security' do
    it { should have_secure_password }
  end

  describe 'roles' do
    it { should define_enum_for(:role).with_values(admin: 0, carer: 1, child: 2).backed_by_column_of_type(:integer) }
  end

  describe 'normalization' do
    it 'downcases the email address before saving' do
      user = User.create(name: 'Test User', date_of_birth: '2000-01-01', email_address: 'TEST@EXAMPLE.COM', password: 'password')
      expect(user.email_address).to eq('test@example.com')
    end
  end
end
