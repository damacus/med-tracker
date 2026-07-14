# frozen_string_literal: true

module MedicationAdministration
  class RestoreHistory
    IMMUTABLE_COLUMNS = %w[
      client_uuid schedule_id person_medication_id taken_at dose_amount dose_unit
      taken_from_medication_id taken_from_location_id
    ].freeze

    def initialize(household:, rows:, conflict_error:)
      @household = household
      @rows = rows
      @conflict_error = conflict_error
    end

    def call
      rows.each { |row| restore(row.with_indifferent_access) }
    end

    private

    attr_reader :household, :rows, :conflict_error

    def restore(row)
      take = MedicationTake.find_or_initialize_by(household: household, portable_id: row.fetch(:portable_id))
      attributes = medication_take_attributes(row)
      verify_immutable!(take, attributes, row) if take.persisted?
      return if take.persisted?

      take.skip_stock_mutation = true
      take.assign_attributes(attributes)
      take.save!
    end

    def verify_immutable!(take, attributes, row)
      expected = MedicationTake.new(attributes).attributes.slice(*IMMUTABLE_COLUMNS)
      actual = take.attributes.slice(*IMMUTABLE_COLUMNS)
      return if actual == expected

      raise conflict_error, "immutable medication take #{row.fetch(:portable_id)} conflicts with the imported record"
    end

    def medication_take_attributes(row)
      medication_take_source_attributes(row).merge(medication_take_event_attributes(row))
                                            .merge(medication_take_inventory_attributes(row))
    end

    def medication_take_source_attributes(row)
      source_type = row.fetch(:source_type)
      source = source_record(source_type, row.fetch(:source_portable_id))

      {
        schedule: source_type == 'schedule' ? source : nil,
        person_medication: source_type == 'person_medication' ? source : nil
      }
    end

    def medication_take_event_attributes(row)
      {
        client_uuid: row[:client_uuid],
        taken_at: row[:taken_at],
        dose_amount: row[:dose_amount],
        dose_unit: row[:dose_unit]
      }
    end

    def medication_take_inventory_attributes(row)
      {
        taken_from_medication: medication_by_portable_id(row[:taken_from_medication_portable_id]),
        taken_from_location: location_by_portable_id(row[:taken_from_location_portable_id])
      }
    end

    def source_record(source_type, portable_id)
      case source_type
      when 'schedule' then Schedule.find_by!(household: household, portable_id: portable_id)
      when 'person_medication' then PersonMedication.find_by!(household: household, portable_id: portable_id)
      else raise conflict_error, 'Unsupported medication take source type'
      end
    end

    def medication_by_portable_id(portable_id)
      return if portable_id.blank?

      Medication.find_by!(household: household, portable_id: portable_id)
    end

    def location_by_portable_id(portable_id)
      return if portable_id.blank?

      Location.find_by!(household: household, portable_id: portable_id)
    end
  end
end
