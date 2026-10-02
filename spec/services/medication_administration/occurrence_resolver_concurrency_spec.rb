require 'rails_helper'
require 'timeout'

RSpec.describe MedicationAdministration::OccurrenceResolver do
  self.use_transactional_tests = false

  fixtures :households, :accounts, :people, :users, :locations, :medications, :dosages, :schedules, :person_medications

  let(:source) { schedules(:john_movicol) }
  let(:membership) { accounts(:admin).household_memberships.find_by!(household: source.household) }
  let(:initial_supply) { source.medication.current_supply }

  before do
    FixtureHouseholdSetup.apply!
    initial_supply
    clear_outcomes
  end

  after do
    clear_outcomes
    source.medication.reload.update!(current_supply: initial_supply)
    source.destroy! if source.is_a?(PersonMedication)
  end

  shared_examples 'concurrent outcome resolution' do
    it 'commits one of a competing take and not-taken decision' do
      results = resolve_concurrently(%w[take unwell])

      expect(results.grep(MedicationDoseOccurrence).size).to eq(1)
      expect(results.grep(described_class::Error).sole.code).to be_in(%w[already_resolved precondition_required])
      expect(source.medication_dose_occurrences.count).to eq(1)
    end

    it 'converges concurrent identical not-taken submissions on one audited outcome' do
      rows = resolve_concurrently(%w[unwell unwell])

      expect(rows).to all(be_a(MedicationDoseOccurrence))
      expect(rows.map(&:id).uniq.size).to eq(1)
      expect(source.medication_dose_occurrences.count).to eq(1)
      expect(rows.first.versions.count).to eq(1)
    end

    it 'commits one of two competing reasons and rejects the other' do
      results = resolve_concurrently(%w[unwell refused])

      expect(results.grep(MedicationDoseOccurrence).size).to eq(1)
      expect(results.grep(described_class::Error).map(&:code)).to eq(['already_resolved'])
      expect(source.medication_dose_occurrences.count).to eq(1)
    end
  end

  it_behaves_like 'concurrent outcome resolution'

  context 'with a direct routine assignment' do
    let(:source) do
      create(:person_medication, :routine, person: people(:john), medication: medications(:vitamin_c),
                                           max_daily_doses: 1)
    end

    it_behaves_like 'concurrent outcome resolution'
  end

  def resolve_concurrently(reasons)
    key = occurrence_key
    ready = Queue.new
    start = Queue.new
    workers = reasons.map { |reason| resolution_worker(key, reason, ready, start) }
    run_workers(workers, ready, start)
  ensure
    workers&.each { |worker| worker.kill if worker.alive? }
  end

  def occurrence_key
    MedicationAdministration::OccurrenceProjection.new(
      source: source, start_date: Date.current, end_date: Date.current
    ).call.first.key
  end

  def run_workers(workers, ready, start)
    workers.size.times { Timeout.timeout(10) { ready.pop } }
    workers.size.times { start << true }
    workers.map { |worker| Timeout.timeout(10) { worker.value } }
  end

  def resolution_worker(key, reason, ready, start)
    source_id = source.id
    membership_id = membership.id
    Thread.new do
      ActiveRecord::Base.connection_pool.with_connection do
        ready << true
        Timeout.timeout(10) { start.pop }
        resolve_in_connection(source_id, membership_id, key, reason)
      end
    rescue StandardError => e
      e
    end
  end

  def resolve_in_connection(source_id, membership_id, key, reason)
    current_source = source.class.find(source_id)
    actor = HouseholdMembership.find(membership_id)
    context = AuthorizationContext.new(account: actor.account, household: current_source.household, membership: actor)
    resolver = described_class.new(source: current_source, authorization: context)
    return resolver.take(key: key, client_uuid: SecureRandom.uuid, if_match: nil) if reason == 'take'

    resolver.call(key: key, action: 'not_taken', reason: reason)
  end

  def clear_outcomes
    take_ids = source.medication_dose_occurrences.pluck(:medication_take_id).compact
    ids = source.medication_dose_occurrences.pluck(:id)
    MedicationDoseOccurrence.where(id: ids).delete_all
    PaperTrail::Version.where(item_type: 'MedicationDoseOccurrence', item_id: ids).delete_all
    MedicationTake.where(id: take_ids).delete_all
    PaperTrail::Version.where(item_type: 'MedicationTake', item_id: take_ids).delete_all
  end
end
