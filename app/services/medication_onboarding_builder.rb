# frozen_string_literal: true

class MedicationOnboardingBuilder
  def initialize(prefill: MedicationOnboardingPrefill.new)
    @prefill = prefill
  end

  def build_new(medication:, params:)
    medication.assign_attributes(finder_prefill_attributes(params))

    defaults = onboarding_prefill_for(
      barcode: medication.barcode,
      code: medication.dmd_code,
      name: medication.name,
      package_quantity: params[:package_quantity],
      package_unit: params[:package_unit]
    )

    defaults.medication_attributes.each do |key, value|
      medication.public_send("#{key}=", value)
    end
    build_onboarding_dosage_records!(medication, defaults.dosage_records_attributes)
    medication
  end

  def merge_create_attributes(attrs)
    defaults = onboarding_prefill_for(
      barcode: attrs[:barcode],
      code: attrs[:dmd_code],
      name: attrs[:name],
      package_quantity: attrs[:package_quantity],
      package_unit: attrs[:package_unit]
    )
    explicit_inventory_override = explicit_inventory_override?(attrs)

    merge_onboarding_medication_defaults!(attrs, defaults.medication_attributes)
    merge_onboarding_dosage_defaults!(
      attrs,
      defaults.dosage_records_attributes,
      explicit_inventory_override: explicit_inventory_override
    )

    attrs
  end

  private

  def finder_prefill_attributes(params)
    finder_identity_attributes(params).merge(finder_code_attributes(params))
  end

  def onboarding_prefill_for(barcode:, code:, name:, package_quantity: nil, package_unit: nil)
    @prefill.call(
      barcode: barcode,
      code: code,
      name: name,
      package_quantity: package_quantity,
      package_unit: package_unit
    )
  end

  def finder_identity_attributes(params)
    attrs = {}
    attrs[:name] = params[:name].presence if params[:name].present?
    attrs[:category] = params[:category].presence if params[:category].present?

    barcode = params[:barcode].presence
    attrs[:barcode] = barcode if NhsDmd::BarcodeLookup.barcode_query?(barcode)
    attrs
  end

  def finder_code_attributes(params)
    return {} if params[:dmd_code].blank?

    {
      dmd_code: params[:dmd_code].presence,
      dmd_system: params[:dmd_system].presence,
      dmd_concept_class: params[:dmd_concept_class].presence
    }
  end

  def dosage_records_blank?(dosage_records_attributes)
    return true if dosage_records_attributes.blank?

    dosage_records_attributes.values.all? do |attributes|
      attributes.except(:id, :_destroy, :default_dose_cycle).values.all?(&:blank?)
    end
  end

  def merge_onboarding_medication_defaults!(attrs, defaults)
    defaults.each do |key, value|
      assign_onboarding_attribute!(attrs, key, value)
    end
  end

  def merge_onboarding_dosage_defaults!(attrs, dosage_defaults, explicit_inventory_override:)
    return unless dosage_records_blank?(attrs[:dosage_records_attributes]) && dosage_defaults.any?

    attrs[:dosage_records_attributes] = serialized_onboarding_dosages(
      dosage_defaults_for_merge(dosage_defaults, explicit_inventory_override:)
    )
  end

  def dosage_defaults_for_merge(dosage_defaults, explicit_inventory_override:)
    return dosage_defaults unless explicit_inventory_override

    dosage_defaults.map { |dosage| dosage.except(:current_supply, :reorder_threshold) }
  end

  def serialized_onboarding_dosages(dosage_defaults)
    dosage_defaults.each_with_index.to_h do |dosage, index|
      [index.to_s, dosage]
    end
  end

  def explicit_inventory_override?(attrs)
    attrs[:current_supply].present? || attrs[:reorder_threshold].present?
  end

  def build_onboarding_dosage_records!(medication, dosage_defaults)
    return if medication.dosage_records.any? || dosage_defaults.blank?

    dosage_defaults.each do |attributes|
      medication.dosage_records.build(attributes)
    end
  end

  def assign_onboarding_attribute!(target, key, value)
    if target.respond_to?(:[])
      target[key] = value if target[key].blank?
      return
    end

    target.public_send("#{key}=", value) if target.public_send(key).blank?
  end
end
