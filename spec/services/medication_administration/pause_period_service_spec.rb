# frozen_string_literal: true

require 'rails_helper'
require 'timeout'

RSpec.describe MedicationAdministration::PausePeriodService do
  self.use_transactional_tests = false

  fixtures :accounts, :households, :schedules, :person_medications

  before do
    FixtureHouseholdSetup.apply!
    reset_lifecycle_records
  end

  after { reset_lifecycle_records }

  let(:household) { households(:fixture_household) }
  let(:membership) { accounts(:admin).household_memberships.find_by!(household:) }
  let(:started_at) { Time.zone.parse('2026-09-02 09:30:00') }

  shared_examples 'a pausable medication source' do
    it 'pauses the source and records one attributable open period' do
      period = described_class.new(
        source:, membership:, reason: 'clinician_advice', note: 'Review in two weeks', started_at:
      ).call

      expect(source.reload).to be_paused
      expect(period).to have_attributes(
        reason: 'clinician_advice',
        note: 'Review in two weeks',
        started_at:,
        ended_at: nil,
        recorded_by_membership: membership,
        legacy_context: false
      )
      expect(source.medication_pause_periods.reload).to contain_exactly(period)
    end

    it 'returns the existing period when stale callers repeat the pause' do
      first_source = source.class.find(source.id)
      second_source = source.class.find(source.id)
      attributes = { membership:, reason: 'side_effects', note: nil, started_at: }

      first_period = described_class.new(source: first_source, **attributes).call
      second_period = described_class.new(source: second_source, **attributes).call

      expect(second_period).to eq(first_period)
      expect(source.medication_pause_periods.where(ended_at: nil).count).to eq(1)
      expect(source.reload).to be_paused
    end

    it 'returns one period when callers pause concurrently' do
      results = concurrent_pause_results

      expect(results).to all(be_a(MedicationPausePeriod))
      expect(results.map(&:id).uniq.one?).to be(true)
      expect(source.medication_pause_periods.where(ended_at: nil).count).to eq(1)
      expect(source.reload).to be_paused
    end

    it 'rolls back the source change when the period is invalid' do
      service = described_class.new(source:, membership:, reason: 'unsupported', note: nil, started_at:)

      expect { service.call }.to raise_error(ActiveRecord::RecordInvalid)
      expect(source.reload.active).to be(true)
      expect(source.medication_pause_periods).to be_empty
    end
  end

  context 'with a Schedule' do
    let(:source) { schedules(:john_paracetamol) }

    it_behaves_like 'a pausable medication source'

    it 'rolls back a created period when the source update fails' do
      original_end_date = source.end_date
      source.end_date = source.start_date - 1.day
      source.save!(validate: false)
      version_count = PaperTrail::Version.where(item_type: 'MedicationPausePeriod').count
      service = described_class.new(
        source:, membership:, reason: 'clinician_advice', note: nil, started_at:
      )

      expect { service.call }.to raise_error(ActiveRecord::RecordInvalid)
      expect(source.reload.active).to be(true)
      expect(source.medication_pause_periods.reload).to be_empty
      expect(PaperTrail::Version.where(item_type: 'MedicationPausePeriod').count).to eq(version_count)
    ensure
      source.end_date = original_end_date
      source.save!(validate: false)
    end
  end

  context 'with a PersonMedication' do
    let(:source) { person_medications(:john_vitamin_d) }

    it_behaves_like 'a pausable medication source'
  end

  def concurrent_pause_results
    ready = Queue.new
    start = Queue.new
    threads = 2.times.map { pause_thread(ready, start) }
    2.times { Timeout.timeout(10) { ready.pop } }
    2.times { start << true }
    threads.map { join_thread(it) }
  ensure
    Array(threads).each(&:kill)
  end

  def pause_thread(ready, start)
    Thread.new do
      ActiveRecord::Base.connection_pool.with_connection { perform_pause(ready, start) }
    rescue StandardError => e
      e
    end
  end

  def perform_pause(ready, start)
    ready << true
    Timeout.timeout(10) { start.pop }
    described_class.new(
      source: source.class.find(source.id),
      membership: HouseholdMembership.find(membership.id),
      reason: 'side_effects',
      note: nil,
      started_at:
    ).call
  end

  def join_thread(thread)
    return thread.value if thread.join(10)

    raise Timeout::Error, 'timed out waiting for pause-period worker'
  end

  def reset_lifecycle_records
    period_ids = lifecycle_period_ids
    MedicationPausePeriod.where(id: period_ids).delete_all
    schedules(:john_paracetamol).update!(active: true)
    person_medications(:john_vitamin_d).update!(active: true)
    PaperTrail::Version.where(item_type: 'MedicationPausePeriod', item_id: period_ids).delete_all
    delete_source_versions
  end

  def lifecycle_period_ids
    MedicationPausePeriod.where(schedule: schedules(:john_paracetamol))
                         .or(MedicationPausePeriod.where(person_medication: person_medications(:john_vitamin_d)))
                         .pluck(:id)
  end

  def delete_source_versions
    PaperTrail::Version.where(item_type: 'Schedule', item_id: schedules(:john_paracetamol).id).delete_all
    versions = PaperTrail::Version.where(
      item_type: 'PersonMedication', item_id: person_medications(:john_vitamin_d).id
    )
    versions.delete_all
  end
end
