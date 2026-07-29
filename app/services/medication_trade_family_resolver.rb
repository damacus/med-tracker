# frozen_string_literal: true

class MedicationTradeFamilyResolver
  BARCODE_JOIN = 'INNER JOIN nhs_dmd_barcodes ' \
                 'ON nhs_dmd_barcodes.gtin IN (' \
                 'medications.barcode, ' \
                 "CASE WHEN char_length(medications.barcode) = 13 THEN '0' || medications.barcode END, " \
                 "CASE WHEN char_length(medications.barcode) = 14 AND left(medications.barcode, 1) = '0' " \
                 'THEN substring(medications.barcode FROM 2) END' \
                 ')'
  AMP_TRADE_FAMILY_JOIN = 'INNER JOIN nhs_dmd_amp_trade_families ' \
                          'ON nhs_dmd_amp_trade_families.amp_code = nhs_dmd_barcodes.amp_code'
  TRADE_FAMILY_JOIN = 'INNER JOIN nhs_dmd_trade_families ' \
                      'ON nhs_dmd_trade_families.id = nhs_dmd_amp_trade_families.trade_family_id'

  def initialize(scope:)
    @scope = scope
  end

  def call(trade_family_codes:)
    codes = Array(trade_family_codes).filter_map(&:presence).uniq
    return {} if codes.empty?

    related_medications(codes).group_by { |medication| medication[:resolved_trade_family_code] }
  end

  private

  attr_reader :scope

  def related_medications(trade_family_codes)
    scope
      .joins(BARCODE_JOIN)
      .joins(AMP_TRADE_FAMILY_JOIN)
      .joins(TRADE_FAMILY_JOIN)
      .where(nhs_dmd_trade_families: { code: trade_family_codes })
      .select('medications.*', 'nhs_dmd_trade_families.code AS resolved_trade_family_code')
      .preload(:location)
      .distinct
  end
end
