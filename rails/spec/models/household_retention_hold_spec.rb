# frozen_string_literal: true

require 'rails_helper'

RSpec.describe HouseholdRetentionHold do
  let(:household) { create(:household) }
  let(:approver) { Account.create!(email: 'hold-model-approver@example.test', status: :verified) }

  it 'requires reason, approver, and a future review date' do
    hold = described_class.new(household: household, placed_at: Time.current, review_on: Date.current)

    expect(hold).not_to be_valid
    expect(hold.errors).to include(:reason, :approved_by_account, :review_on)
  end

  it 'keeps preservation evidence immutable after placement' do
    hold = described_class.create!(
      household: household,
      approved_by_account: approver,
      reason: 'Approved preservation',
      review_on: 1.month.from_now.to_date,
      placed_at: Time.current
    )

    hold.reason = 'Changed reason'
    hold.approved_by_account = Account.create!(email: 'replacement-approver@example.test', status: :verified)
    hold.review_on = 2.months.from_now.to_date

    expect(hold).not_to be_valid
    expect(hold.errors[:base]).to include('Retention hold evidence is immutable')
  end

  it 'keeps release evidence immutable after release' do
    releaser = Account.create!(email: 'hold-model-releaser@example.test', status: :verified)
    hold = described_class.create!(
      household: household,
      approved_by_account: approver,
      reason: 'Approved preservation',
      review_on: 1.month.from_now.to_date,
      placed_at: Time.current
    )
    hold.update!(status: :released, released_by_account: releaser, released_at: Time.current)

    hold.assign_attributes(status: :active, released_by_account: nil, released_at: nil)

    expect(hold).not_to be_valid
    expect(hold.errors[:base]).to include('Retention hold release evidence is immutable')
  end
end
