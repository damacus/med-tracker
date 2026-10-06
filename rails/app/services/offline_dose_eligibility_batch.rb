# frozen_string_literal: true

class OfflineDoseEligibilityBatch
  def initialize(sources:, user:, now: Time.current, &policy_for)
    @sources = sources
    @user = user
    @now = now
    @policy_for = policy_for
  end

  def call
    return {} if sources.empty?

    preload_timing_history
    sources.index_with do |source|
      context = OfflineDoseEligibility::Context.new(record_access: record_access?(source),
                                                    resolver: resolvers.fetch(source),
                                                    decision_context: decisions.fetch(source))
      OfflineDoseEligibility.new(source: source, user: user, now: now, context: context)
    end
  end

  private

  attr_reader :sources, :user, :now, :policy_for

  def resolvers
    @resolvers ||= MedicationStockSourceResolver.new(user: user, source: sources.first, taken_at: now).preload(sources)
  end

  def decisions
    @decisions ||= MedicationDoseDecisionContext.new(source: sources.first, taken_at: now).preload(sources)
  end

  def preload_timing_history
    ActiveRecord::Associations::Preloader.new(
      records: sources,
      associations: :medication_takes,
      scope: MedicationTake.where(taken_at: (now - 31.days).beginning_of_day..now.end_of_day)
    ).call
  end

  def record_access?(source)
    @record_access ||= {}
    key = [source.class, source.person_id]
    return @record_access[key] if @record_access.key?(key)

    @record_access[key] = policy_for.call(source).take_medication?
  end
end
