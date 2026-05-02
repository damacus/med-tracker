# frozen_string_literal: true

class MedicationStockSourceResolver
  attr_reader :user, :source

  def self.bulk_fetch(user, sources)
    # Filter out sources with nil medication to avoid NoMethodError
    valid_sources = sources.select { |s| s.medication.present? }
    return {} if valid_sources.empty?

    # 1. Collect unique medication attributes
    keys = valid_sources.map do |s|
      m = s.medication
      [m.name, m.dosage_amount, m.dosage_unit]
    end.uniq

    # 2. Build a query that covers all these combinations
    query = Medication.none
    keys.each do |key|
      query = query.or(Medication.where(
                         name: key[0],
                         dosage_amount: key[1],
                         dosage_unit: key[2]
                       ))
    end

    # 3. Apply the same scope and ordering as MedicationStockSourceResolver
    scope = if user.blank?
              Medication.where(id: valid_sources.map(&:medication_id))
            else
              MedicationPolicy::Scope.new(user, Medication.all).resolve
            end

    all_matching = scope.merge(query)
                        .joins(:location)
                        .includes(:location)
                        .order('locations.name ASC, medications.id ASC')
                        .to_a

    # 4. Group by key for fast lookup
    results = all_matching.group_by { |m| [m.name, m.dosage_amount, m.dosage_unit] }
    # Ensure all requested keys are present in the hash even if no medications were found
    keys.each { |key| results[key] ||= [] }
    results
  end

  def initialize(user:, source:, matching_medications: nil)
    @user = user
    @source = source
    @matching_medications = matching_medications
  end

  def available_medications
    @available_medications ||= matching_medications.reject(&:out_of_stock?)
  end

  def blocked_reason
    return :out_of_stock if available_medications.empty?
    return :cooldown unless source.can_take_now?

    nil
  end

  def resolve_selected(taken_from_medication_id)
    return available_medications.first if taken_from_medication_id.blank? && available_medications.one?

    medication = matching_medications.find do |candidate|
      candidate.id == taken_from_medication_id.to_i
    end

    return if medication.blank? || medication.out_of_stock?

    medication
  end

  def selection_required?(taken_from_medication_id)
    taken_from_medication_id.blank? && available_medications.many?
  end

  private

  def matching_medications
    @matching_medications ||= begin
      medication = source.medication
      return [] if medication.blank?

      resolved_scope
        .joins(:location)
        .includes(:location)
        .where(
          name: medication.name,
          dosage_amount: medication.dosage_amount,
          dosage_unit: medication.dosage_unit
        )
        .order('locations.name ASC, medications.id ASC')
        .to_a
    end
  end

  def resolved_scope
    return Medication.where(id: source.medication_id) if user.blank?

    MedicationPolicy::Scope.new(user, Medication.all).resolve
  end
end
