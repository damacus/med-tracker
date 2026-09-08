module PortableData
  module ImportWriterPausePeriods
    def import_medication_pause_periods
      rows = records(:medication_pause_periods)
      rows.sort_by { |row| row[:ended_at].present? ? 0 : 1 }.each { |row| restore_pause_period(row) }
      return unless payload[:format] == 'medtracker.portable.v2'

      reconcile_pause_sources(rows)
    end

    def restore_pause_period(row)
      source = source_record(row.fetch(:source_type), row.fetch(:source_portable_id))
      source.lock!
      period = prepare_pause_period(row, source)
      verify_open_pause!(period, source)
      period.save! if period.changed?
    end

    def prepare_pause_period(row, source)
      period = find_or_initialize(MedicationPausePeriod, row)
      attributes = pause_attributes(row, source)
      verify_existing_pause!(period, attributes, row)
      assign_pause_attributes(period, attributes, row) unless period.persisted? && !period.imported_context?
      period
    end

    def verify_existing_pause!(period, attributes, row)
      return unless period.persisted?

      verify_pause_identity!(period, attributes)
      verify_pause_actors!(period, row)
    end

    def assign_pause_attributes(period, attributes, row)
      attributes[:recorded_by_membership] ||= period.recorded_by_membership
      attributes[:resumed_by_membership] ||= period.resumed_by_membership
      period.assign_attributes(attributes)
      period.created_at = row[:created_at] if period.new_record? && row[:created_at].present?
    end

    def pause_attributes(row, source)
      row.slice(:reason, :note, :started_at, :ended_at, :legacy_context).merge(
        schedule: source.is_a?(Schedule) ? source : nil,
        person_medication: source.is_a?(PersonMedication) ? source : nil,
        recorded_by_membership: pause_actor(row[:recorded_by_person_portable_id]),
        resumed_by_membership: pause_actor(row[:resumed_by_person_portable_id]),
        imported_context: true,
        imported_actor_references: row.slice(:recorded_by_person_portable_id, :resumed_by_person_portable_id)
      )
    end

    def pause_actor(portable_id)
      return if portable_id.blank?

      matches = household.household_memberships.joins(:person).where(people: { portable_id: portable_id }).limit(2)
      matches.first if matches.size == 1
    end

    def verify_pause_identity!(period, attributes)
      columns = %w[schedule_id person_medication_id reason note started_at legacy_context]
      columns << 'ended_at' unless period.imported_context? && period.ended_at.nil?
      expected = MedicationPausePeriod.new(attributes).attributes.slice(*columns)
      return if period.attributes.slice(*columns) == expected

      invalid_pause!(period, 'Imported pause conflicts with recorded history')
    end

    def verify_open_pause!(period, source)
      return if period.ended_at.present?
      return unless source.medication_pause_periods.where(ended_at: nil).where.not(id: period.id).exists?

      invalid_pause!(period, 'Source already has an open pause period')
    end

    def verify_pause_actors!(period, row)
      actors = period.imported_context? && !period.ended_at? ? %i[recorded_by] : %i[recorded_by resumed_by]
      actors.each do |actor|
        field = "#{actor}_person_portable_id"
        original = existing_pause_actor_reference(period, actor, field)
        invalid_pause!(period, 'Imported pause conflicts with recorded actors') unless original == row[field]
      end
    end

    def existing_pause_actor_reference(period, actor, field)
      period.public_send("#{actor}_membership")&.person&.portable_id || period.imported_actor_references[field]
    end

    def invalid_pause!(period, message)
      period.errors.add(:base, message)
      raise ActiveRecord::RecordInvalid, period
    end

    def reconcile_pause_sources(rows)
      pause_source_references(rows).uniq.sort.each do |type, portable_id|
        source = source_record(type, portable_id)
        source.lock!
        reconcile_pause_source(source, type, portable_id)
      end
    end

    def pause_source_references(rows)
      references = rows.map { |row| [row[:source_type], row[:source_portable_id]] }
      references.concat(records(:schedules).map { |row| ['schedule', row[:portable_id]] })
      references.concat(records(:person_medications).map { |row| ['person_medication', row[:portable_id]] })
    end

    def reconcile_pause_source(source, type, portable_id)
      open = source.medication_pause_periods.exists?(ended_at: nil)
      return if source.retired_at.present? && !open

      validate_imported_pause_state!(source, type, portable_id) unless open
      source.update!(active: !open) if source.active == open
    end

    def validate_imported_pause_state!(source, type, portable_id)
      collection = type == 'schedule' ? :schedules : :person_medications
      row = records(collection).find { |record| record[:portable_id] == portable_id }
      return unless row&.key?(:active) && !ActiveModel::Type::Boolean.new.cast(row[:active])

      invalid_pause!(source, 'Inactive source requires an open pause period')
    end
  end
end
