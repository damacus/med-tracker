# frozen_string_literal: true

class GlobalSearchQuery
  include Rails.application.routes.url_helpers

  Result = Data.define(:type, :title, :subtitle, :path, :score) do
    def as_json(*)
      {
        type: type,
        title: title,
        subtitle: subtitle,
        path: path,
        score: score
      }
    end
  end

  TYPE_ORDER = {
    'command' => 0,
    'person' => 1,
    'medication' => 2,
    'person_medication' => 3,
    'schedule' => 4,
    'location' => 5
  }.freeze

  attr_reader :user, :query, :limit

  def initialize(user:, query:, limit: 12)
    @user = user
    @query = query.to_s.strip
    @limit = limit.to_i.positive? ? limit.to_i : 12
  end

  def call
    return [] unless user
    return command_results if query.blank?

    results = matching_command_results
    results.concat(record_results) if query.length >= 2
    sort_results(results)
  end

  private

  def record_results
    person_results + medication_results + location_results + schedule_results + person_medication_results
  end

  def person_results
    scoped(Person)
      .where('people.name ILIKE ?', search_term)
      .order(:name)
      .limit(limit)
      .map do |person|
        build_result(
          type: 'person',
          title: person.name,
          subtitle: I18n.t('global_search.types.person'),
          path: person_path(person)
        )
      end
  end

  def medication_results
    scoped(Medication)
      .includes(:location)
      .where(medication_search_sql, term: search_term)
      .order(:name)
      .limit(limit)
      .map do |medication|
        build_result(
          type: 'medication',
          title: medication.name,
          subtitle: medication_subtitle(medication),
          path: medication_path(medication),
          secondary_values: [medication.category, medication.barcode, medication.dmd_code]
        )
      end
  end

  def location_results
    scoped(Location)
      .where('locations.name ILIKE ?', search_term)
      .order(:name)
      .limit(limit)
      .map do |location|
        build_result(
          type: 'location',
          title: location.name,
          subtitle: I18n.t('global_search.types.location'),
          path: location_path(location)
        )
      end
  end

  def schedule_results
    scoped(Schedule)
      .joins(:person, :medication)
      .includes(:person, :medication)
      .where(schedule_search_sql, term: search_term)
      .order('medications.name ASC', 'people.name ASC')
      .limit(limit)
      .map { |schedule| schedule_result(schedule) }
  end

  def person_medication_results
    scoped(PersonMedication)
      .joins(:person, :medication)
      .includes(:person, :medication)
      .where(person_medication_search_sql, term: search_term)
      .order('medications.name ASC', 'people.name ASC')
      .limit(limit)
      .map { |person_medication| person_medication_result(person_medication) }
  end

  def schedule_result(schedule)
    build_result(
      type: 'schedule',
      title: I18n.t('global_search.result_titles.schedule', medication: schedule.medication.name),
      subtitle: schedule_subtitle(schedule),
      path: person_path(schedule.person, anchor: "schedule_#{schedule.id}"),
      secondary_values: [schedule.person.name]
    )
  end

  def person_medication_result(person_medication)
    build_result(
      type: 'person_medication',
      title: person_medication.medication.name,
      subtitle: person_medication_subtitle(person_medication),
      path: person_path(person_medication.person, anchor: "person_medication_#{person_medication.id}"),
      secondary_values: [person_medication.person.name]
    )
  end

  def medication_search_sql
    'medications.name ILIKE :term OR medications.category ILIKE :term OR ' \
      'medications.barcode ILIKE :term OR medications.dmd_code ILIKE :term'
  end

  def schedule_search_sql
    'medications.name ILIKE :term OR people.name ILIKE :term'
  end

  def person_medication_search_sql
    'medications.name ILIKE :term OR people.name ILIKE :term'
  end

  def medication_subtitle(medication)
    [medication.category.presence, medication.location&.name].compact.join(' · ').presence ||
      I18n.t('global_search.types.medication')
  end

  def schedule_subtitle(schedule)
    I18n.t(
      'global_search.subtitles.schedule',
      person: schedule.person.name,
      frequency: schedule.frequency.presence || I18n.t('global_search.subtitles.no_frequency')
    )
  end

  def person_medication_subtitle(person_medication)
    I18n.t('global_search.subtitles.person_medication', person: person_medication.person.name)
  end

  def build_result(type:, title:, subtitle:, path:, secondary_values: [])
    Result.new(
      type: type,
      title: title,
      subtitle: subtitle,
      path: path,
      score: score_for(title, secondary_values)
    )
  end

  def command_results
    GlobalSearchCommandsQuery.new(user: user).call
  end

  def matching_command_results
    command_results.filter_map do |result|
      score = score_for(result.title, [result.subtitle])
      next unless score.positive?

      Result.new(
        type: result.type,
        title: result.title,
        subtitle: result.subtitle,
        path: result.path,
        score: score
      )
    end
  end

  def scoped(model)
    Pundit.policy_scope!(user, model)
  end

  def score_for(title, secondary_values)
    normalized_title = normalize(title)
    return 100 if normalized_title == normalized_query
    return 80 if normalized_title.start_with?(normalized_query)
    return 60 if normalized_title.include?(normalized_query)
    return 40 if secondary_values.any? { |value| normalize(value).include?(normalized_query) }

    0
  end

  def sort_results(results)
    results
      .select { |result| result.score.positive? }
      .sort_by { |result| [-result.score, TYPE_ORDER.fetch(result.type), result.title.downcase] }
      .first(limit)
  end

  def normalize(value)
    value.to_s.strip.downcase
  end

  def normalized_query
    @normalized_query ||= normalize(query)
  end

  def search_term
    @search_term ||= "%#{ActiveRecord::Base.sanitize_sql_like(query)}%"
  end
end
