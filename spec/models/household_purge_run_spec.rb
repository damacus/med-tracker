# frozen_string_literal: true

require 'rails_helper'

RSpec.describe HouseholdPurgeRun do
  let(:household) { create(:household) }
  let(:operator) do
    Account.create!(email: 'purge-run-operator@example.test', status: :verified)
  end

  it 'acquires one durable run for a household across repeated commands' do
    first_run = described_class.acquire!(household: household, requested_by_account: operator)
    second_run = described_class.acquire!(household: household, requested_by_account: operator)

    expect(second_run).to eq(first_run)
    expect(described_class.where(household: household).count).to eq(1)
  end

  it 'rejects a second run for the same household at the model boundary' do
    described_class.create!(household: household, requested_by_account: operator)
    duplicate = described_class.new(household: household, requested_by_account: operator)

    expect(duplicate).not_to be_valid
    expect(duplicate.errors[:household_id]).to include('has already been taken')
  end

  it 'enforces one run per household in PostgreSQL' do
    described_class.create!(household: household, requested_by_account: operator)

    expect { insert_duplicate_run }.to raise_error(ActiveRecord::RecordNotUnique)
  end

  it 'declares the household index unique in the schema' do
    household_index = ActiveRecord::Base.connection.indexes(:household_purge_runs).find do |index|
      index.columns == ['household_id']
    end

    expect(household_index.unique).to be(true)
  end

  def insert_duplicate_run
    connection = ActiveRecord::Base.connection
    timestamp = connection.quote(Time.current)
    connection.execute(<<~SQL.squish)
      INSERT INTO household_purge_runs
        (household_id, requested_by_account_id, created_at, updated_at)
      VALUES (#{household.id}, #{operator.id}, #{timestamp}, #{timestamp})
    SQL
  end
end
