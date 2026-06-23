# frozen_string_literal: true

module GlobalSearch
  class SchedulesResultsQuery < RecordResultsQuery
    def call
      scoped(Schedule)
        .joins(:person, :medication)
        .includes(:person, :medication)
        .where(search_sql, term: search_term)
        .order('medications.name ASC', 'people.name ASC')
        .limit(limit)
        .map { |schedule| result_for(schedule) }
    end

    private

    def result_for(schedule)
      builder.build(
        type: 'schedule',
        title: I18n.t('global_search.result_titles.schedule', medication: schedule.medication.display_name),
        subtitle: subtitle_for(schedule),
        path: person_path(*tenant_route_args(schedule.person), anchor: "schedule_#{schedule.id}"),
        secondary_values: [schedule.person.name]
      )
    end

    def subtitle_for(schedule)
      I18n.t(
        'global_search.subtitles.schedule',
        person: schedule.person.name,
        frequency: schedule.frequency.presence || I18n.t('global_search.subtitles.no_frequency')
      )
    end

    def search_sql
      'medications.name ILIKE :term OR people.name ILIKE :term'
    end
  end
end
