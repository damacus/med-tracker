# frozen_string_literal: true

class MedicationDoseModeSynchronizer
  def initialize(medication)
    @medication = medication
  end

  def call
    affected_person_medications = medication.person_medications.where.not(source_dosage_option_id: nil).to_a
    deleted_dosages = MedicationDosageOption.where(medication: medication).to_a
    binds = medication_id_binds
    clear_person_medication_sources(binds)
    delete_dosages(binds)
    deleted_dosages.each(&:record_sync_deletion!)
    affected_person_medications.each(&:refresh_sync_version!)
  end

  private

  attr_reader :medication

  def medication_id_binds
    [
      ActiveRecord::Relation::QueryAttribute.new(
        'medication_id',
        medication.id,
        ActiveRecord::Type::BigInteger.new
      )
    ]
  end

  def clear_person_medication_sources(binds)
    ActiveRecord::Base.connection.exec_update(
      'UPDATE person_medications SET source_dosage_option_id = NULL WHERE medication_id = $1',
      'Sync Person Medication Dosage Sources',
      binds
    )
  end

  def delete_dosages(binds)
    ActiveRecord::Base.connection.exec_delete('DELETE FROM dosages WHERE medication_id = $1', 'Sync Dosages', binds)
  end
end
