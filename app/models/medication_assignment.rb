# frozen_string_literal: true

class MedicationAssignment
  include ActiveModel::Model
  include ActiveModel::Attributes

  attribute :medication_id, :integer
  attribute :source_dosage_option_id, :integer
  attribute :dose_amount, :decimal
  attribute :dose_unit, :string
end
