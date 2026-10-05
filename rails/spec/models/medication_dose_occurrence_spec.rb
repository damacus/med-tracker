require 'rails_helper'

RSpec.describe MedicationDoseOccurrence do
  fixtures :households, :accounts, :people, :locations, :medications, :dosages, :schedules,
           :person_medications, :medication_takes

  subject(:occurrence) do
    described_class.new(schedule: schedules(:john_paracetamol), window_starts_on: Date.current, position: 1)
  end

  let(:household) { households(:fixture_household) }
  let(:membership) do
    household.household_memberships.find_or_create_by!(account: accounts(:admin)) do |record|
      record.role = :owner
      record.status = :active
    end
  end

  def resolve_not_taken
    occurrence.assign_attributes(outcome: 'not_taken', resolved_at: Time.current,
                                 resolved_by_membership: membership, reason: 'refused')
  end

  describe 'state validation' do
    it 'allows an open occurrence with a source household and portable identity' do
      occurrence.save!

      expect(occurrence.reload).to have_attributes(household: household, outcome: 'open')
      expect(occurrence.portable_id).to be_present
    end

    it 'requires exactly one concrete source' do
      occurrence.schedule = nil
      expect(occurrence).not_to be_valid

      occurrence.schedule = schedules(:john_paracetamol)
      occurrence.person_medication = person_medications(:john_vitamin_d)
      expect(occurrence).not_to be_valid
    end

    it 'reserves storage for the direct source without requiring a schedule' do
      occurrence.assign_attributes(schedule: nil, person_medication: person_medications(:john_vitamin_d))

      expect(occurrence).to be_valid
    end

    it 'requires a date and positive integral position' do
      occurrence.assign_attributes(window_starts_on: nil, position: 0)

      expect(occurrence).not_to be_valid
      expect(occurrence.errors.attribute_names).to include(:window_starts_on, :position)
    end

    it 'requires a supported outcome' do
      occurrence.outcome = 'missed'

      expect(occurrence).not_to be_valid
    end

    it 'accepts supported not-taken reasons and optional context' do
      resolve_not_taken
      [nil, *described_class::REASONS].each do |reason|
        occurrence.reason = reason
        expect(occurrence).to be_valid
      end
    end

    it 'rejects unsupported reasons and excessive notes' do
      resolve_not_taken
      occurrence.assign_attributes(reason: 'invented', note: 'x' * 2001)

      expect(occurrence).not_to be_valid
      expect(occurrence.errors.attribute_names).to include(:reason, :note)
    end

    it 'requires attribution for a resolved outcome' do
      occurrence.outcome = 'not_taken'

      expect(occurrence).not_to be_valid
      expect(occurrence.errors.attribute_names).to include(:resolved_at, :resolved_by_membership)
    end

    it 'does not allow resolution context on an open occurrence' do
      resolve_not_taken
      occurrence.outcome = 'open'

      expect(occurrence).not_to be_valid
    end

    it 'requires the matching immutable take for a taken outcome' do
      occurrence.assign_attributes(outcome: 'taken', resolved_at: Time.current,
                                   resolved_by_membership: membership)
      expect(occurrence).not_to be_valid

      occurrence.medication_take = medication_takes(:john_morning_paracetamol)
      expect(occurrence).to be_valid

      occurrence.medication_take = medication_takes(:jane_morning_ibuprofen)
      expect(occurrence).not_to be_valid
    end

    it 'does not allow a take on a not-taken outcome' do
      resolve_not_taken
      occurrence.medication_take = medication_takes(:john_morning_paracetamol)

      expect(occurrence).not_to be_valid
    end

    it 'rejects a source or actor from another household' do
      occurrence.household = Household.new(id: household.id + 1)
      expect(occurrence).not_to be_valid

      occurrence.household = household
      resolve_not_taken
      occurrence.resolved_by_membership = HouseholdMembership.new(household_id: household.id + 1)
      expect(occurrence).not_to be_valid
    end
  end

  describe 'retained identity and history' do
    it 'refuses migration rollback so saved clinical outcomes remain available' do
      load Rails.root.join('db/migrate/20260908200000_create_medication_dose_occurrences.rb') unless
        defined?(CreateMedicationDoseOccurrences)
      resolve_not_taken
      occurrence.save!

      ActiveRecord::Base.transaction(requires_new: true) do
        ActiveRecord::Base.connection.execute('SET LOCAL ROLE med_tracker_owner')
        expect { CreateMedicationDoseOccurrences.new.down }.to raise_error(ActiveRecord::IrreversibleMigration)
        ActiveRecord::Base.connection.execute('RESET ROLE')
      end

      expect(occurrence.reload).to have_attributes(outcome: 'not_taken', reason: 'refused')
    ensure
      ActiveRecord::Base.connection.execute('RESET ROLE')
    end

    it 'retains the former not-taken context in audit history when reopened' do
      resolve_not_taken
      occurrence.save!

      expect do
        occurrence.update!(outcome: 'open', reason: nil, note: nil, resolved_at: nil, resolved_by_membership: nil)
      end.to change(PaperTrail::Version, :count).by(1)
      expect(occurrence.versions.last.reify.reason).to eq('refused')
    end

    it 'prevents changes to a persisted natural or portable identity' do
      occurrence.save!
      occurrence.assign_attributes(position: 2, window_starts_on: Date.yesterday, portable_id: SecureRandom.uuid)

      expect(occurrence).not_to be_valid
      expect(occurrence.errors.attribute_names).to include(:position, :window_starts_on, :portable_id)
    end

    it 'prevents a linked outcome from reopening' do
      occurrence.assign_attributes(outcome: 'taken', medication_take: medication_takes(:john_morning_paracetamol),
                                   resolved_at: Time.current, resolved_by_membership: membership)
      occurrence.save!
      occurrence.assign_attributes(outcome: 'open', medication_take: nil, resolved_at: nil,
                                   resolved_by_membership: nil)

      expect(occurrence).not_to be_valid
    end

    it 'prevents deletion of a source with retained occurrence history' do
      occurrence.save!

      expect(occurrence.schedule.destroy).to be(false)
      expect(occurrence.reload).to be_persisted
    end
  end

  describe 'database integrity' do
    def insert_invalid(attributes)
      occurrence.save!
      described_class.transaction(requires_new: true) { occurrence.update_columns(attributes) }
    ensure
      occurrence.reload
    end

    it 'rejects invalid source and state combinations without model validation' do
      expect { insert_invalid(schedule_id: nil) }.to raise_error(ActiveRecord::StatementInvalid)
      expect { insert_invalid(outcome: 'taken') }.to raise_error(ActiveRecord::StatementInvalid)
      expect { insert_invalid(position: 0) }.to raise_error(ActiveRecord::StatementInvalid)
    end

    it 'enforces a unique identity per concrete source and window' do
      occurrence.save!
      duplicate = occurrence.dup
      duplicate.portable_id = SecureRandom.uuid

      expect do
        described_class.transaction(requires_new: true) { duplicate.save!(validate: false) }
      end.to raise_error(ActiveRecord::RecordNotUnique)
    end

    it 'prevents the same take from satisfying two occurrences' do
      occurrence.assign_attributes(outcome: 'taken', medication_take: medication_takes(:john_morning_paracetamol),
                                   resolved_at: Time.current, resolved_by_membership: membership)
      occurrence.save!
      duplicate = occurrence.dup
      duplicate.assign_attributes(position: 2, portable_id: SecureRandom.uuid)

      expect do
        described_class.transaction(requires_new: true) { duplicate.save!(validate: false) }
      end.to raise_error(ActiveRecord::RecordNotUnique)
    end

    it 'uses composite household foreign keys for sources, take and actor' do
      columns = ActiveRecord::Base.connection.foreign_keys(:medication_dose_occurrences).map do |foreign_key|
        foreign_key.options[:column]
      end

      expect(columns).to include(%w[schedule_id household_id], %w[person_medication_id household_id],
                                 %w[medication_take_id household_id], %w[resolved_by_membership_id household_id])
    end

    it 'forces household row-level security' do
      row = ActiveRecord::Base.connection.select_one(
        "SELECT relrowsecurity, relforcerowsecurity FROM pg_class WHERE oid = 'medication_dose_occurrences'::regclass"
      )

      expect(row).to include('relrowsecurity' => true, 'relforcerowsecurity' => true)
    end
  end
end
