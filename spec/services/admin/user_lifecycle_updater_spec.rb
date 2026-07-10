# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Admin::UserLifecycleUpdater do
  fixtures :accounts, :households, :people, :users

  before { FixtureHouseholdSetup.apply! }

  let(:household) { households(:fixture_household) }
  let(:actor) { users(:admin) }
  let(:actor_membership) do
    household.household_memberships.find_by!(account: actor.person.account)
  end

  def update_lifecycle(user, action, actor_user: actor, membership: actor_membership)
    described_class.new(
      user: user,
      action: action,
      actor: actor_user,
      household: household,
      actor_membership: membership
    ).call
  end

  it 'activates an inactive user' do
    user = users(:jane)
    user.deactivate!

    result = update_lifecycle(user, :activate)

    expect(result).to have_attributes(success?: true, message: I18n.t('users.activated'))
    expect(user.reload).to be_active
  end

  it 'deactivates an active member' do
    user = users(:jane)

    result = update_lifecycle(user, :deactivate)

    expect(result).to have_attributes(success?: true, message: I18n.t('users.deactivated'))
    expect(user.reload).not_to be_active
  end

  it 'rejects self-deactivation' do
    result = update_lifecycle(actor, :deactivate)

    expect(result).to have_attributes(success?: false, message: I18n.t('users.cannot_deactivate_self'))
    expect(actor.reload).to be_active
  end

  it 'rejects owner deactivation by a household administrator' do
    owner = users(:parent)
    administrator = users(:jane)
    assign_role(owner, :owner)
    administrator_membership = assign_role(administrator, :administrator)

    result = update_lifecycle(
      owner,
      :deactivate,
      actor_user: administrator,
      membership: administrator_membership
    )

    expect(result).to have_attributes(
      success?: false,
      message: I18n.t('users.owner_deactivation_rejected')
    )
    expect(owner.reload).to be_active
  end

  it 'verifies the linked account and removes verification keys' do
    user = users(:jane)
    account = user.person.account
    account.update!(status: :unverified)
    AccountVerificationKey.create!(account_id: account.id, key: 'verification-key')

    result = update_lifecycle(user, :verify)

    expect(result).to have_attributes(success?: true, message: I18n.t('users.verified'))
    expect(account.reload).to be_verified
    expect(AccountVerificationKey.where(account_id: account.id)).to be_empty
  end

  it 'rejects verification when the user has no linked account' do
    person = Person.create!(
      household: household,
      name: 'Accountless User',
      date_of_birth: '1990-01-01'
    )
    user = User.create!(person: person, email_address: 'accountless@example.test')

    result = update_lifecycle(user, :verify)

    expect(result).to have_attributes(
      success?: false,
      message: I18n.t('admin.users.missing_account')
    )
  end

  def assign_role(user, role)
    household.household_memberships.find_by!(account: user.person.account).tap do |membership|
      membership.update!(role: role)
    end
  end
end
