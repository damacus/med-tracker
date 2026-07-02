# frozen_string_literal: true

class MedicationDoseSource
  TYPES = {
    Schedule => 'schedule',
    PersonMedication => 'person_medication'
  }.freeze

  attr_reader :record

  delegate :person, :medication, :household, to: :record
  delegate :id, to: :record, prefix: true

  def self.for(take)
    record = take.schedule || take.person_medication
    return unless record

    new(record)
  end

  def initialize(record)
    @record = record
    raise ArgumentError, "Unsupported medication dose source: #{record.class.name}" unless type
  end

  def type
    TYPES[record.class]
  end
end
