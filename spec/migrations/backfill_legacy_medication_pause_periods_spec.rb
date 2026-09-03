# frozen_string_literal: true

require 'rails_helper'
require 'timeout'

unless defined?(BackfillLegacyMedicationPausePeriods)
  load Rails.root.join('db/migrate/20260902100000_backfill_legacy_medication_pause_periods.rb')
end

RSpec.describe BackfillLegacyMedicationPausePeriods do
  it 'creates one unknown-start legacy period for each uncovered inactive source' do
    inactive_schedule = create(:schedule, active: false)
    inactive_assignment = create(:person_medication, active: false)
    active_schedule = create(:schedule, active: true)
    active_assignment = create(:person_medication, active: true)
    retired_schedule = create(:schedule, active: false, retired_at: 1.day.ago)
    retired_assignment = create(:person_medication, active: false, retired_at: 1.day.ago)

    described_class.new.up

    expect_legacy_period(inactive_schedule.medication_pause_periods.sole)
    expect_legacy_period(inactive_assignment.medication_pause_periods.sole)
    expect(active_schedule.medication_pause_periods).to be_empty
    expect(active_assignment.medication_pause_periods).to be_empty
    expect(retired_schedule.medication_pause_periods).to be_empty
    expect(retired_assignment.medication_pause_periods).to be_empty
  end

  it 'retains existing open and completed history' do
    covered_assignment = create(:person_medication, active: false)
    existing_open = covered_assignment.medication_pause_periods.create!(
      reason: MedicationPausePeriod::LEGACY_REASON,
      started_at: nil,
      recorded_by_membership: nil,
      legacy_context: true
    )
    inactive_schedule = create(:schedule, active: false)
    completed_period = create_completed_period(inactive_schedule)
    completed_attributes = completed_period.attributes

    described_class.new.up

    expect(covered_assignment.medication_pause_periods.reload).to contain_exactly(existing_open)
    expect(completed_period.reload.attributes).to eq(completed_attributes)
    expect(inactive_schedule.medication_pause_periods.where(ended_at: nil).count).to eq(1)
    expect(inactive_schedule.medication_pause_periods.count).to eq(2)
  end

  it 'is safe to rerun' do
    inactive_schedule = create(:schedule, active: false)
    inactive_assignment = create(:person_medication, active: false)

    2.times { described_class.new.up }

    expect(inactive_schedule.medication_pause_periods.where(ended_at: nil).count).to eq(1)
    expect(inactive_assignment.medication_pause_periods.where(ended_at: nil).count).to eq(1)
  end

  def expect_legacy_period(period)
    expect(period).to have_attributes(
      reason: MedicationPausePeriod::LEGACY_REASON,
      started_at: nil,
      ended_at: nil,
      recorded_by_membership: nil,
      resumed_by_membership: nil,
      legacy_context: true
    )
  end

  def create_completed_period(source)
    account = Account.create!(email: "pause-backfill-#{SecureRandom.hex(4)}@example.test", status: :verified)
    membership = source.household.household_memberships.create!(account:, role: :owner, status: :active)
    source.medication_pause_periods.create!(
      reason: 'side_effects',
      started_at: 2.days.ago,
      ended_at: 1.day.ago,
      recorded_by_membership: membership,
      resumed_by_membership: membership
    )
  end

  context 'when resume overlaps reconciliation' do
    self.use_transactional_tests = false

    fixtures :accounts, :households, :schedules, :person_medications

    let(:source) { schedules(:john_paracetamol) }
    let(:membership) { accounts(:admin).household_memberships.find_by!(household: source.household) }
    let(:existing_period_ids) { MedicationPausePeriod.pluck(:id) }
    let(:existing_version_ids) { PaperTrail::Version.pluck(:id) }

    before do
      FixtureHouseholdSetup.apply!
      existing_period_ids
      existing_version_ids
      source.update!(active: false)
    end

    after { cleanup_concurrency_records }

    it 'does not leave an active source with an open period' do
      run_overlapping_resume

      expect(source.reload).to be_active
      expect(source.medication_pause_periods.where(ended_at: nil)).to be_empty
    end
  end

  def run_overlapping_resume
    resume_thread, release_resume = start_locked_resume
    migration_thread = start_blocked_migration
    release_resume << true
    thread_value(resume_thread)
    thread_value(migration_thread)
  ensure
    stop_workers(release_resume, resume_thread, migration_thread)
  end

  def start_locked_resume
    ready = Queue.new
    release = Queue.new
    thread = start_resume_thread(ready, release)
    Timeout.timeout(10) { ready.pop }
    [thread, release]
  end

  def start_blocked_migration
    backend_pid = Queue.new
    thread = start_migration_thread(backend_pid)
    wait_until_blocked(Timeout.timeout(10) { backend_pid.pop })
    thread
  end

  def stop_workers(release, resume_thread, migration_thread)
    release&.push(true)
    resume_thread&.kill
    migration_thread&.kill
  end

  def start_resume_thread(ready, release)
    Thread.new { ActiveRecord::Base.connection_pool.with_connection { resume_under_lock(ready, release) } }
  end

  def resume_under_lock(ready, release)
    source.class.transaction do
      resumable_source = source.class.lock.find(source.id)
      ready << true
      Timeout.timeout(10) { release.pop }
      resume(resumable_source)
    end
  end

  def resume(resumable_source)
    MedicationAdministration::ResumePeriodService.new(
      source: resumable_source,
      membership: HouseholdMembership.find(membership.id),
      ended_at: Time.current
    ).call
  end

  def start_migration_thread(backend_pid)
    Thread.new do
      ActiveRecord::Base.connection_pool.with_connection do |connection|
        backend_pid << connection.select_value('SELECT pg_backend_pid()')
        described_class.new.up
      end
    end
  end

  def wait_until_blocked(backend_pid)
    Timeout.timeout(10) do
      loop do
        blocking_count = ActiveRecord::Base.connection.select_value(
          "SELECT cardinality(pg_blocking_pids(#{Integer(backend_pid)}))"
        )
        break if blocking_count.positive?

        Thread.pass
      end
    end
  end

  def thread_value(thread)
    raise Timeout::Error, 'timed out waiting for migration worker' unless thread.join(10)

    thread.value
  end

  def cleanup_concurrency_records
    MedicationPausePeriod.where.not(id: existing_period_ids).delete_all
    source.update!(active: true) unless source.active?
    PaperTrail::Version.where.not(id: existing_version_ids).delete_all
  end
end
