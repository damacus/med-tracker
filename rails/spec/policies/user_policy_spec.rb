# frozen_string_literal: true

require 'rails_helper'
require 'pundit/rspec'

RSpec.describe UserPolicy, type: :policy do
  subject(:policy) { described_class.new(current_user, target_user) }

  let(:household_member) { household_policy_member(role: :owner) }
  let(:household) { household_member.fetch(:household) }
  let(:current_user) { household_member.fetch(:context) }
  let(:target_user) { user_for_household(household, 'target-user@example.test') }

  context 'with a household manager context' do
    it 'permits user administration' do
      %i[index show create new update edit destroy activate verify].each do |action|
        expect(policy.public_send("#{action}?")).to be true
      end
    end
  end

  context 'with a household member context' do
    let(:current_user) { household_policy_member(role: :member, household: household).fetch(:context) }

    it 'forbids user administration' do
      %i[index show create new update edit destroy activate verify].each do |action|
        expect(policy.public_send("#{action}?")).to be false
      end
    end
  end

  context 'with a legacy administrator user' do
    let(:current_user) { User.new }

    it 'forbids user administration' do
      %i[index show create new update edit destroy activate verify].each do |action|
        expect(policy.public_send("#{action}?")).to be false
      end
    end
  end

  describe 'Scope' do
    subject(:scope) { described_class::Scope.new(current_user, User.all).resolve }

    let!(:household_user) { target_user }
    let!(:other_user) do
      user_for_household(household_policy_member(role: :owner).fetch(:household), 'other-user@example.test')
    end

    context 'with a household manager context' do
      it 'returns users whose accounts belong to the household' do
        expect(scope).to include(household_user)
        expect(scope).not_to include(other_user)
      end
    end

    context 'with a household member context' do
      let(:current_user) { household_policy_member(role: :member, household: household).fetch(:context) }

      it 'returns no users' do
        expect(scope).to be_empty
      end
    end
  end

  def user_for_household(household, email)
    account = Account.create!(email: email, status: :verified)
    person = household.people.create!(
      name: email.split('@').first.titleize,
      email: email,
      date_of_birth: 30.years.ago.to_date,
      person_type: :adult,
      has_capacity: true,
      account: account
    )
    household.household_memberships.create!(account: account, person: person, role: :member, status: :active)
    User.create!(person: person, email_address: email, active: true)
  end
end
