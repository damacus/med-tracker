module MedicationAdministration
  class OccurrenceResolver
    class Error < StandardError
      attr_reader :code

      def initialize(message, code: 'invalid_occurrence')
        @code = code
        super(message)
      end
    end

    def initialize(source:, authorization:)
      @source = source
      @authorization = authorization
    end

    def call(key:, action:, reason: nil, note: nil, if_match: nil)
      Households::LifecycleCutoffLock.with(household_id: source.household_id) do
        source.with_lock do
          authorize_action!(action)
          row = find_occurrence(key)
          case action
          when 'not_taken' then resolve_not_taken(row, reason: reason, note: note)
          when 'reopen' then reopen(row, if_match: if_match)
          else raise Error, 'Unsupported outcome action'
          end
        end
      end
    end

    def take(key:, taken_at: Time.current, **)
      Households::LifecycleCutoffLock.with(household_id: source.household_id) do
        source.with_lock do
          authorize_action!('take')
          administer(find_occurrence(key), taken_at: taken_at, **)
        end
      end
    end

    private

    attr_reader :source, :authorization

    def administer(row, taken_at:, **options)
      return replay_take(row.record, options[:client_uuid]) if row.record&.taken?

      validate_take_version!(row.record, options)
      validate_actionable!(row)
      unless taken_at.in_time_zone.to_date == row.window_starts_on
        raise Error, 'Dose does not match the occurrence window'
      end

      link_take(row, record_dose(taken_at: taken_at, **options))
    end

    def record_dose(taken_at:, **options)
      result = RecordDose.new.call(source: source, user: authorization, taken_at: taken_at,
                                   amount_override: options[:dose_amount],
                                   taken_from_medication_id: options[:taken_from_medication_id],
                                   client_uuid: options[:client_uuid], route: options[:route])
      raise Error.new('Dose could not be recorded', code: result.error.to_s) unless result.success

      result.take
    end

    def link_take(row, medication_take)
      record = unresolved_record(row)
      record.update!(outcome: 'taken', medication_take: medication_take, reason: nil, note: nil,
                     resolved_at: Time.current, resolved_by_membership: authorization.membership)
      record
    end

    def replay_take(record, client_uuid)
      return record if client_uuid.present? && record.medication_take.client_uuid == client_uuid

      raise Error.new('Occurrence is already resolved', code: 'already_resolved')
    end

    def authorize_action!(action)
      raise Error, 'Occurrence is unavailable' unless source.household.reload.operational?

      current_context = authorization.with(membership: authorization.membership.reload)
      Pundit.authorize(current_context, source, action == 'reopen' ? :update? : :take_medication?)
    end

    def find_occurrence(key)
      identity = OccurrenceProjection.decode(key)
      raise Error, 'Occurrence is unavailable' unless matching_identity?(identity)

      date = Date.iso8601(identity[2])
      rows = OccurrenceProjection.new(source: source, start_date: date, end_date: date).call
      rows.find { |row| row.position == identity[3] } || raise(Error, 'Occurrence is unavailable')
    rescue Date::Error, TypeError
      raise Error, 'Occurrence is unavailable'
    end

    def matching_identity?(identity)
      identity.is_a?(Array) && identity.size == 4 && identity[0] == 'schedule' &&
        identity[1] == source.portable_id && identity[3].is_a?(Integer)
    end

    def resolve_not_taken(row, reason:, note:)
      return replay(row.record, reason: reason, note: note) if row.record && !row.record.open?

      validate_actionable!(row)
      record = unresolved_record(row)
      record.update!(outcome: 'not_taken', reason: reason, note: note,
                     resolved_at: Time.current, resolved_by_membership: authorization.membership)
      record
    end

    def validate_actionable!(row)
      raise Error.new('Occurrence is already resolved', code: 'already_resolved') if row.legacy_take
      raise Error, 'Occurrence is unavailable' unless row.expected?
      raise Error, 'Occurrence is not due' unless row.due?
    end

    def unresolved_record(row)
      row.record || source.medication_dose_occurrences.build(
        window_starts_on: row.window_starts_on, position: row.position, scheduled_at: row.scheduled_at
      )
    end

    def replay(record, reason:, note:)
      return record if record.not_taken? && record.reason == reason && record.note == note

      raise Error.new('Occurrence is already resolved', code: 'already_resolved')
    end

    def reopen(row, if_match:)
      record = row.record
      raise Error, 'Occurrence cannot be reopened' unless record&.not_taken?

      validate_version!(record, if_match)

      record.update!(outcome: 'open', reason: nil, note: nil, resolved_at: nil, resolved_by_membership: nil)
      record
    end

    def validate_take_version!(record, options)
      return unless options.key?(:if_match)
      return if !record&.not_taken? && options[:if_match].blank?

      validate_version!(record, options[:if_match])
    end

    def validate_version!(record, if_match)
      raise Error.new('A current version is required', code: 'precondition_required') if if_match.blank?
      return if record && if_match == Api::RecordEtag.for(record)

      raise Error.new('Occurrence has changed', code: 'sync_conflict')
    end
  end
end
