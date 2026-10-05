# frozen_string_literal: true

module HealthEvents
  class PatternSummary
    Summary = Data.define(
      :normalized_title,
      :display_title,
      :episode_count,
      :average_duration_days,
      :average_interval_days,
      :first_started_on,
      :most_recent_started_on
    )

    attr_reader :events

    def initialize(events:)
      @events = events
    end

    def call
      grouped_events.filter_map do |normalized_title, group|
        next unless group.size > 1

        sorted_group = group.sort_by(&:started_on)
        Summary.new(
          normalized_title: normalized_title,
          display_title: sorted_group.first.title.to_s.squish,
          episode_count: group.size,
          average_duration_days: average_duration_days(sorted_group),
          average_interval_days: average_interval_days(sorted_group),
          first_started_on: sorted_group.first.started_on,
          most_recent_started_on: sorted_group.last.started_on
        )
      end
    end

    private

    def grouped_events
      events.group_by { |event| normalize_title(event.title) }.reject { |title, _group| title.blank? }
    end

    def normalize_title(title)
      title.to_s.squish.downcase
    end

    def average_duration_days(group)
      durations = group.filter_map do |event|
        next if event.ended_on.blank?

        (event.ended_on - event.started_on).to_i + 1
      end
      return if durations.empty?

      average_rounded(durations)
    end

    def average_interval_days(group)
      intervals = group.each_cons(2).map do |previous_event, next_event|
        (next_event.started_on - previous_event.started_on).to_i
      end
      return if intervals.empty?

      average_rounded(intervals)
    end

    def average_rounded(values)
      (values.sum.to_f / values.size).round
    end
  end
end
