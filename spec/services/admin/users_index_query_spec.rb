# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Admin::UsersIndexQuery do
  fixtures :accounts, :people, :users, :locations, :location_memberships

  let(:scope) { User.all }
  let!(:soft_deleted_user) do
    account = Account.create!(
      email: 'soft.deleted@example.com',
      password_hash: RodauthApp.rodauth.allocate.password_hash('password'),
      status: :closed
    )
    person = Person.create!(
      household: Household.first,
      name: 'Soft Deleted User',
      date_of_birth: '1990-01-01',
      account: account
    )
    person.update!(account: nil)
    User.create!(
      person: person,
      email_address: 'soft.deleted@example.com',
      active: true
    )
  end

  describe '#call' do
    it 'searches by person name or email' do
      result = described_class.new(scope: scope, filters: { search: 'Jane' }).call

      expect(result).to include(users(:jane))
      expect(result).not_to include(users(:bob))
    end

    it 'filters by household membership role without using the user role' do
      household = users(:jane).person.household
      attach_user_to_household(users(:jane), household, :administrator)
      attach_user_to_household(users(:doctor), household, :member)

      result = described_class.new(
        scope: scope.where(id: [users(:jane).id, users(:doctor).id]),
        filters: { membership_role: 'administrator' },
        household: household
      ).call

      expect(result).to contain_exactly(users(:jane))
    end

    it 'filters soft deleted users' do
      result = described_class.new(scope: scope, filters: { status: 'soft_deleted' }).call

      expect(result).to contain_exactly(soft_deleted_user)
    end

    it 'filters active users' do
      users(:bob).update!(active: false)

      result = described_class.new(
        scope: scope.where(id: [users(:jane).id, users(:bob).id]),
        filters: { status: 'active' }
      ).call

      expect(result).to contain_exactly(users(:jane))
    end

    it 'filters inactive users' do
      users(:bob).update!(active: false)

      result = described_class.new(
        scope: scope.where(id: [users(:jane).id, users(:bob).id]),
        filters: { status: 'inactive' }
      ).call

      expect(result).to contain_exactly(users(:bob))
    end

    it 'leaves the scope unchanged for unknown status filters' do
      result = described_class.new(
        scope: scope.where(id: [users(:jane).id, users(:bob).id]),
        filters: { status: 'unknown' }
      ).call

      expect(result).to contain_exactly(users(:jane), users(:bob))
    end

    it 'returns no users when filtering by membership role without a household' do
      result = described_class.new(
        scope: scope.where(id: users(:jane).id),
        filters: { membership_role: 'member' }
      ).call

      expect(result).to be_empty
    end

    it 'sorts by associated person name' do
      result = described_class.new(
        scope: scope.where(id: [users(:jane).id, users(:bob).id]),
        filters: { sort: 'name', direction: 'asc' }
      ).call

      expect(result.map(&:name)).to eq(['Bob Smith', 'Jane Doe'])
    end

    it 'sorts by email when requested' do
      result = described_class.new(
        scope: scope.where(id: [users(:jane).id, users(:bob).id]),
        filters: { sort: 'email', direction: 'desc' }
      ).call

      expect(result.map(&:email_address)).to eq(['jane.doe@example.com', 'bob.smith@example.com'])
    end

    it 'sorts by household membership role when requested' do
      household = users(:jane).person.household
      attach_user_to_household(users(:jane), household, :member)
      attach_user_to_household(users(:doctor), household, :administrator)

      result = described_class.new(
        scope: scope.where(id: [users(:jane).id, users(:doctor).id]),
        filters: { sort: 'membership_role', direction: 'asc' },
        household: household
      ).call

      expect(result.map(&:email_address)).to eq(['dr.jones@example.com', 'jane.doe@example.com'])
    end
  end

  def attach_user_to_household(user, household, role)
    membership = household.household_memberships.find_or_create_by!(account: user.person.account, person: user.person)
    membership.update!(role: role, status: :active)
  end
end
