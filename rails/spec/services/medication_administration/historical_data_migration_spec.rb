# frozen_string_literal: true

require 'rails_helper'

RSpec.describe MedicationAdministration::HistoricalDataMigration do
  subject(:migration) { described_class.new }

  let(:legacy_household) { create(:household) }
  let(:source_location) { create(:location, household: legacy_household) }
  let(:target_location) { create(:location, household: legacy_household) }
  let(:schedule) do
    medication = create(:medication, household: legacy_household, location: source_location)
    create(:schedule, household: legacy_household, person: create(:person, household: legacy_household), medication:)
  end
  let!(:take) do
    create(
      :medication_take,
      :for_schedule,
      household: legacy_household,
      schedule: schedule,
      taken_from_medication: schedule.medication,
      taken_from_location: source_location,
      skip_stock_mutation: true
    )
  end

  it 'backfills only history without a household and is idempotent' do
    household = create(:household)
    relation = instance_double(ActiveRecord::Relation)
    connection = take.class.connection
    allow(MedicationTake).to receive(:where).with(household_id: nil).and_return(relation)
    allow(relation).to receive(:find_each).and_yield(take)
    allow(connection).to receive(:execute)

    expect do
      2.times { migration.backfill_household(household:) }
    end.to change(take, :household_id).from(legacy_household.id).to(household.id)

    expect(connection).to have_received(:execute).twice
  end

  it 'moves only history associated with the legacy location and is idempotent' do
    retained = create(:medication_take, :for_schedule, household: legacy_household, schedule: schedule,
                                                       skip_stock_mutation: true)

    expect do
      2.times { migration.move_location(from: source_location, into: target_location) }
    end.to change { take.reload.taken_from_location }.from(source_location).to(target_location)

    expect(retained.reload.taken_from_location).to be_nil
  end

  it 'participates in its caller transaction' do
    ActiveRecord::Base.transaction do
      migration.move_location(from: source_location, into: target_location)
      raise ActiveRecord::Rollback
    end

    expect(take.reload.taken_from_location).to eq(source_location)
  end
end
