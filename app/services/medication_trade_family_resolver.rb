class MedicationTradeFamilyResolver
  def initialize(scope:)
    @scope = scope
  end

  def call(trade_family_code:, excluding: nil)
    return scope.none if trade_family_code.blank?

    related_medications = scope
      .joins(
        'INNER JOIN nhs_dmd_barcodes ' \
        'ON nhs_dmd_barcodes.gtin = medications.barcode'
      )
      .joins(
        'INNER JOIN nhs_dmd_amp_trade_families ' \
        'ON nhs_dmd_amp_trade_families.amp_code = nhs_dmd_barcodes.amp_code'
      )
      .joins(
        'INNER JOIN nhs_dmd_trade_families ' \
        'ON nhs_dmd_trade_families.id = nhs_dmd_amp_trade_families.trade_family_id'
      )
      .where(nhs_dmd_trade_families: { code: trade_family_code })
      .includes(:location)
      .distinct

    excluding ? related_medications.where.not(id: excluding.id) : related_medications
  end

  private

  attr_reader :scope
end
