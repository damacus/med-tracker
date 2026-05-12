# frozen_string_literal: true

class MedicationStockMatchResolver
  def initialize(scope:, barcode_lookup: BarcodeCatalog::Lookup.new)
    @scope = scope
    @barcode_lookup = barcode_lookup
  end

  def call(**attributes)
    normalized_barcode = NhsDmdBarcode.normalize_gtin(attributes[:barcode])
    direct_match = direct_barcode_match(normalized_barcode)
    return direct_match if direct_match

    catalog_match = catalog_match_for(normalized_barcode)
    candidate = candidate_for(match_attributes(attributes, normalized_barcode, catalog_match))
    return if candidate.blank?

    MedicationInventoryMatcher.new(scope: scope).call(candidate)
  end

  private

  attr_reader :scope, :barcode_lookup

  def direct_barcode_match(barcode)
    return if barcode.blank?

    scope.find_by(barcode: barcode)
  end

  def catalog_match_for(barcode)
    return if barcode.blank?

    barcode_lookup.lookup(barcode)
  end

  def match_attributes(attributes, barcode, catalog_match)
    {
      barcode: barcode,
      code: attributes[:code] || catalog_match&.fetch(:code, nil),
      system: attributes[:system] || catalog_match&.fetch(:system, nil),
      concept_class: attributes[:concept_class] || catalog_match&.fetch(:concept_class, nil),
      name: attributes[:name] || attributes[:display] || catalog_match&.fetch(:display, nil),
      package_unit: attributes[:package_unit]
    }
  end

  def candidate_for(attributes)
    return if attributes.values_at(:barcode, :code, :name).all?(&:blank?)

    Medication.new(
      name: attributes[:name],
      barcode: attributes[:barcode],
      dmd_code: attributes[:code],
      dmd_system: attributes[:system],
      dmd_concept_class: attributes[:concept_class],
      dosage_unit: attributes[:package_unit]
    )
  end
end
