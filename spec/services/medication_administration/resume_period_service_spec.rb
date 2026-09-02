# frozen_string_literal: true

require 'rails_helper'
require 'timeout'

RSpec.describe MedicationAdministration::ResumePeriodService do
  self.use_transactional_tests = false

  fixtures :accounts, :households, :schedules, :person_medications

  before do
    FixtureHouseholdSetup.apply!
    reset_lifecycle_records
  end

  after { reset_lifecycle_records }

  let(:household) { households(:fixture_household) }
  let(:membership) { accounts(:admin).household_memberships.find_by!(household:) }
  let(:started_at) { Time.zone.parse('2026-09-01 09:30:00') }
  let(:ended_at) { Time.zone.parse('2026-09-02 09:30:00') }

  shared_examples 'a resumable medication source' do
    it 'activates the source and closes its open period without losing context' do
      period = create_open_period

      result = described_class.new(source:, membership:, ended_at:).call

      expect(source.reload).to be_active
      expect(result.reload).to have_attributes(
        id: period.id,
        reason: 'side_effects',
        note: 'Keep this context',
        started_at:,
        ended_at:,
        recorded_by_membership: membership,
        resumed_by_membership: membership
      )
    end

    it 'returns the completed period when stale callers repeat the resume' do
      period = create_open_period
      first_source = source.class.find(source.id)
      second_source = source.class.find(source.id)

      first_result = described_class.new(source: first_source, membership:, ended_at:).call
      second_result = described_class.new(source: second_source, membership:, ended_at: ended_at + 1.hour).call

      expect(second_result).to eq(first_result)
      expect(period.reload.ended_at).to eq(ended_at)
      expect(source.reload).to be_active
    end

    it 'closes one period when callers resume concurrently' do
      period = create_open_period

      results = concurrent_resume_results

      expect(results).to all(be_a(MedicationPausePeriod))
      expect(results.map(&:id).uniq).to eq([period.id])
      expect(period.reload).to have_attributes(ended_at:, resumed_by_membership: membership)
      expect(source.reload).to be_active
    end

    it 'records unknown legacy context before resuming an inactive source without a period' do
      source.update!(active: false)

      period = described_class.new(source:, membership:, ended_at:).call

      expect(source.reload).to be_active
      expect(period).to have_attributes(
        reason: MedicationPausePeriod::LEGACY_REASON,
        started_at: nil,
        ended_at:,
        recorded_by_membership: nil,
        resumed_by_membership: membership,
        legacy_context: true
      )
    end

    it 'does not invent history when an active source is resumed' do
      result = described_class.new(source:, membership:, ended_at:).call

      expect(result).to be_nil
      expect(source.reload).to be_active
      expect(source.medication_pause_periods).to be_empty
    end
  end

  context 'with a Schedule' do
    let(:source) { schedules(:john_paracetamol) }

    it_behaves_like 'a resumable medication source'

    it 'rolls back a closed period when the source update fails' do
      period = create_open_period
      original_end_date = source.end_date
      source.end_date = source.start_date - 1.day
      source.save!(validate: false)
      version_count = PaperTrail::Version.where(item_type: 'MedicationPausePeriod', item_id: period.id).count

      expect { described_class.new(source:, membership:, ended_at:).call }
        .to raise_error(ActiveRecord::RecordInvalid)
      expect(source.reload).to be_paused
      expect(period.reload).to have_attributes(ended_at: nil, resumed_by_membership: nil)
      expect(PaperTrail::Version.where(item_type: 'MedicationPausePeriod', item_id: period.id).count)
        .to eq(version_count)
    ensure
      source.end_date = original_end_date
      source.save!(validate: false)
    end
  end

  context 'with a PersonMedication' do
    let(:source) { person_medications(:john_vitamin_d) }

    it_behaves_like 'a resumable medication source'
  end

  def create_open_period
    source.update!(active: false)
    source.medication_pause_periods.create!(
      reason: 'side_effects', note: 'Keep this context', started_at:, recorded_by_membership: membership
    )
  end

  def concurrent_resume_results
    ready = Queue.new
    start = Queue.new
    threads = 2.times.map { resume_thread(ready, start) }
    2.times { Timeout.timeout(10) { ready.pop } }
    2.times { start << true }
    threads.map { join_thread(it) }
  ensure
    Array(threads).each(&:kill)
  end

  def resume_thread(ready, start)
    Thread.new do
      ActiveRecord::Base.connection_pool.with_connection { perform_resume(ready, start) }
    rescue StandardError => e
      e
    end
  end

  def perform_resume(ready, start)
    ready << true
    Timeout.timeout(10) { start.pop }
    described_class.new(
      source: source.class.find(source.id),
      membership: HouseholdMembership.find(membership.id),
      ended_at:
    ).call
  end

  def join_thread(thread)
    return thread.value if thread.join(10)

    raise Timeout::Error, 'timed out waiting for resume-period worker'
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
