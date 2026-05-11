# frozen_string_literal: true

module SmartInsights
  class IndexQuery
    MINIMUM_EVIDENCE_DAYS = 7
    MINIMUM_EVENTS = 5

    DETECTORS = [
      Detectors::InventoryRisk,
      Detectors::MissedDosePattern,
      Detectors::AdherenceStreak,
      Detectors::TimingConsistency,
      Detectors::PrnUsage,
      Detectors::ScheduleHygiene
    ].freeze

    SEVERITY_PRIORITY = {
      urgent: 0,
      warning: 1,
      positive: 2,
      info: 3
    }.freeze

    INSIGHT_PRIORITY = {
      inventory_risk: 0,
      missed_dose_pattern: 1,
      adherence_streak: 2,
      timing_consistency: 3,
      prn_usage: 4,
      schedule_hygiene: 5
    }.freeze

    attr_reader :people, :start_date, :end_date

    def initialize(people:, start_date:, end_date:)
      @people = people
      @start_date = start_date
      @end_date = end_date
    end

    def call
      return learning_result unless context.enough_evidence?

      insights = detected_insights
      Result.new(
        primary_insight: insights.first,
        insights: insights,
        learning_state?: false,
        evidence_summary: context.evidence_summary
      )
    end

    private

    def context
      @context ||= Context.new(people: people, start_date: start_date, end_date: end_date)
    end

    def learning_result
      Result.new(
        primary_insight: nil,
        insights: [],
        learning_state?: true,
        evidence_summary: context.evidence_summary
      )
    end

    def detected_insights
      DETECTORS.flat_map { |detector| detector.new(context).call }
               .sort_by { |insight| [SEVERITY_PRIORITY.fetch(insight.severity), INSIGHT_PRIORITY.fetch(insight.key)] }
    end
  end
end
