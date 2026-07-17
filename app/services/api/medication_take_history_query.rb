# frozen_string_literal: true

module Api
  class MedicationTakeHistoryQuery
    Result = Data.define(:records, :meta)
    INCLUDES = [
      { schedule: %i[person medication] },
      { person_medication: %i[person medication] },
      :taken_from_location,
      :taken_from_medication
    ].freeze

    def self.cursor_mode?(params)
      %i[cursor person_id from to].any? { |key| params[key].present? }
    end

    def initialize(scope:, visible_people:, household:, params:)
      @scope = scope
      @visible_people = visible_people
      @household = household
      @params = params
    end

    def call
      rows = paginated_rows
      has_more = rows.size > per_page
      records = rows.first(per_page)

      Result.new(records: records, meta: history_meta(records, has_more))
    rescue MedicationTakeHistoryCursor::Invalid
      raise Api::V1::BaseController::InvalidFilterValue, 'cursor is invalid'
    end

    private

    attr_reader :scope, :visible_people, :household, :params

    def paginated_rows
      relation = apply_cursor(filter_scope.reorder(taken_at: :desc, id: :desc))
      relation.includes(*INCLUDES).limit(per_page + 1).to_a
    end

    def history_meta(records, has_more)
      {
        per_page: per_page,
        next_cursor: has_more ? cursor.encode(records.last, filter_digest:) : nil,
        has_more: has_more
      }
    end

    def filter_scope
      raise_invalid_range if from_time && to_time && from_time > to_time

      filter_to(filter_from(filter_by_person(scope)))
    end

    def filter_from(relation)
      return relation unless from_time

      relation.where(relation.klass.arel_table[:taken_at].gteq(from_time))
    end

    def filter_to(relation)
      return relation unless to_time

      relation.where(relation.klass.arel_table[:taken_at].lteq(to_time))
    end

    def filter_by_person(relation)
      return relation if params[:person_id].blank?

      person = resolved_person
      schedule_ids = Schedule.where(household:, person:).select(:id)
      person_medication_ids = PersonMedication.where(household:, person:).select(:id)

      history_for_sources(relation, schedule_ids, person_medication_ids)
    end

    def resolved_person
      Api::PortableRecordLocator.new(household:).find(visible_people, params[:person_id])
    rescue ActiveRecord::RecordNotFound
      raise Api::V1::BaseController::InvalidFilterValue, 'person_id is invalid'
    end

    def history_for_sources(relation, schedule_ids, person_medication_ids)
      scheduled = relation.where(schedule_id: schedule_ids)
      scheduled.or(relation.where(person_medication_id: person_medication_ids))
    end

    def from_time
      @from_time ||= parse_time_filter(:from)
    end

    def to_time
      @to_time ||= parse_time_filter(:to)
    end

    def parse_time_filter(name)
      value = params[name]
      return if value.blank?

      Time.iso8601(value.to_s)
    rescue ArgumentError
      raise Api::V1::BaseController::InvalidFilterValue, "#{name} must be ISO8601"
    end

    def raise_invalid_range
      raise Api::V1::BaseController::InvalidFilterValue, 'from must be before or equal to to'
    end

    def apply_cursor(relation)
      return relation if params[:cursor].blank?

      taken_at, id = cursor.decode(params[:cursor], filter_digest:)
      table = relation.klass.arel_table
      relation.where(cursor_predicate(table, taken_at, id))
    end

    def cursor_predicate(table, taken_at, id)
      table[:taken_at].lt(taken_at).or(
        table[:taken_at].eq(taken_at).and(table[:id].lt(id))
      )
    end

    def cursor
      @cursor ||= MedicationTakeHistoryCursor.new(household:)
    end

    def filter_digest
      @filter_digest ||= Digest::SHA256.hexdigest(
        JSON.generate([resolved_person_filter, from_time&.iso8601(6), to_time&.iso8601(6)])
      )
    end

    def resolved_person_filter
      return if params[:person_id].blank?

      resolved_person.portable_id
    end

    def per_page
      @per_page ||= params.fetch(:per_page, 20).to_i.clamp(1, 100)
    end
  end
end
