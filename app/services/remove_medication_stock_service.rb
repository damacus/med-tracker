class RemoveMedicationStockService
  REASONS = %w[dropped damaged expired discarded lost transferred_out other].freeze
  EVENT_TYPE = 'MedicationStockRemoval'.freeze
  Result = Data.define(:success, :error) do
    def success?
      success
    end
  end

  def self.history(medication)
    PaperTrail::Version.where(item_type: EVENT_TYPE, item_id: medication.id, household_id: medication.household_id,
                              event: 'stock_removal')
  end

  def self.find_removal(medication, submission_id)
    history(medication).where(
      "CASE WHEN item_type = 'MedicationStockRemoval' THEN object::jsonb ->> 'submission_id' END = ?",
      submission_id
    ).first
  end

  def call(medication:, **attributes)
    @medication = medication
    @quantity = parse_quantity(attributes[:quantity])
    return failure(:invalid_quantity) unless valid_quantity?

    @payload = build_payload(attributes)
    error = validation_error
    return failure(error) if error

    ActiveRecord::Base.transaction(requires_new: true) { remove_locked_stock }
  end

  private

  attr_reader :medication, :quantity, :payload

  def build_payload(attributes)
    { 'quantity' => MedicationStockQuantityFormatter.format(quantity), 'reason' => attributes[:reason].to_s,
      'note' => attributes[:note].to_s.strip, 'submission_id' => attributes[:submission_id].to_s,
      'dosage_id' => attributes[:dosage_id].to_s }
  end

  def validation_error
    return :invalid_reason unless REASONS.include?(payload['reason'])
    return :invalid_note if payload['note'].size > 1000
    return :invalid_submission unless payload['submission_id'].match?(/\A[0-9a-f]{8}-(?:[0-9a-f]{4}-){3}[0-9a-f]{12}\z/)

    :invalid_source unless payload['dosage_id'].match?(/\A(?:[1-9][0-9]*)?\z/)
  end

  def parse_quantity(value)
    BigDecimal(value.to_s)
  rescue ArgumentError
    nil
  end

  def valid_quantity?
    quantity&.finite? && quantity.positive? && quantity.round(2) == quantity
  end

  def remove_locked_stock
    dosage = selected_dosage
    return failure(:invalid_source) if payload['dosage_id'].present? && dosage.nil?

    dosage&.lock!
    medication.lock!
    previous_event = replay_event
    return replay_result(previous_event) if previous_event

    apply_removal(dosage)
  end

  def selected_dosage
    medication.dosage_records.find_by(id: payload['dosage_id']) if payload['dosage_id'].present?
  end

  def apply_removal(dosage)
    stock = dosage || medication
    error = stock_error(stock, dosage)
    return failure(error) if error

    previous = stock.current_supply
    decrement(stock, dosage)
    record_event(stock, previous)
    Result.new(success: true, error: nil)
  end

  def stock_error(stock, dosage)
    return :invalid_source if dosage.nil? && medication.dosage_records.where.not(current_supply: nil).exists?
    return :untracked if stock.current_supply.nil?

    :insufficient_stock if stock.current_supply < quantity
  end

  def replay_event
    self.class.find_removal(medication, payload['submission_id'])
  end

  def replay_result(event)
    previous = JSON.parse(event.object).slice(*payload.keys)
    return failure(:changed_submission) unless previous == payload

    Result.new(success: true, error: nil)
  end

  def decrement(stock, dosage)
    if dosage
      dosage.with_inventory_sync_suppressed { dosage.update!(current_supply: stock.current_supply - quantity) }
      medication.sync_inventory_from_dosage_records!
    else
      medication.update!(current_supply: stock.current_supply - quantity)
    end
  end

  def record_event(stock, previous)
    unit = stock.is_a?(MedicationDosageOption) ? stock.unit : medication.dose_unit
    unit = 'units' unless MedicationStockConsumption.stock_quantity_unit?(unit)
    Audit::VersionEvent.record!(
      item_type: EVENT_TYPE, item_id: medication.id, event: 'stock_removal', household_id: medication.household_id,
      object: payload.merge('previous_quantity' => MedicationStockQuantityFormatter.format(previous),
                            'remaining_quantity' => MedicationStockQuantityFormatter.format(stock.current_supply),
                            'unit' => unit)
    )
  end

  def failure(error)
    Result.new(success: false, error: error)
  end
end
