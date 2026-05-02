# frozen_string_literal: true

FactoryBot.define do
  factory :nhs_dmd_barcode do
    gtin { '05016298210989' }
    code { '13629411000001105' }
    display { 'Laxido Orange oral powder sachets (Galen Ltd)' }
    system { 'https://dmd.nhs.uk' }
    concept_class { 'AMPP' }
  end
end
