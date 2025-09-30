# frozen_string_literal: true

require 'rails_helper'

RSpec.describe User do
  subject(:user) do
    described_class.new(
      email_address: 'test@example.com',
      password: 'password',
      password_confirmation: 'password'
    )
  end

  let(:person) do
    Person.new(
      name: 'Person One',
      email: 'person.one@example.com',
      date_of_birth: Date.new(2000, 1, 1)
    )
  end

  describe 'validations' do
    it { is_expected.to validate_presence_of(:email_address) }
    it { is_expected.to validate_uniqueness_of(:email_address).case_insensitive }
    it { is_expected.to allow_value('user@example.com').for(:email_address) }
    it { is_expected.not_to allow_value('user@example').for(:email_address) }
    it { is_expected.not_to allow_value('userexample.com').for(:email_address) }
    it { is_expected.to validate_presence_of(:person) }
  end

  describe 'associations' do
    it { is_expected.to have_many(:sessions).dependent(:destroy) }
    it { is_expected.to belong_to(:person).inverse_of(:user) }
    it { is_expected.to have_many(:prescriptions).through(:person) }
  end

  describe 'person linkage' do
    it 'requires an associated person' do
      expect(user).to validate_presence_of(:person)
    end
  end

  describe 'callbacks' do
    it 'assigns the person association before validation' do
      user.person = person
      expect(user.person).to eq(person)
    end
  end

  describe 'security' do
    it { is_expected.to have_secure_password }
  end

  describe 'roles' do
    it {
      expect(user).to define_enum_for(:role).with_values(admin: 0, carer: 1,
                                                         child: 2).backed_by_column_of_type(:integer)
    }
  end

  describe 'normalization' do
    it 'downcases the email address before saving' do
      user = described_class.create(email_address: 'TEST@EXAMPLE.COM', password: 'password', person: person)
      expect(user.email_address).to eq('test@example.com')
    end
  end
end
