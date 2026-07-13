# frozen_string_literal: true

module PortableData
  module ImportWriterMedicationTakes
    def import_medication_takes
      records(:medication_takes).each do |row|
        take = find_or_initialize(MedicationTake, row)
        attributes = medication_take_attributes(row)
        verify_immutable_medication_take!(take, attributes, row) if take.persisted?
        next if take.persisted?

        take.skip_stock_mutation = true
        take.assign_attributes(attributes)
        take.save!
      end
    end

    def verify_immutable_medication_take!(take, attributes, row)
      expected = MedicationTake.new(attributes).attributes.slice(*immutable_medication_take_columns)
      actual = take.attributes.slice(*immutable_medication_take_columns)
      return if actual == expected

      raise Importer::Error, "immutable medication take #{row.fetch(:portable_id)} conflicts with the imported record"
    end

    def immutable_medication_take_columns
      %w[
        client_uuid schedule_id person_medication_id taken_at dose_amount dose_unit
        taken_from_medication_id taken_from_location_id
      ]
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
  end
end
