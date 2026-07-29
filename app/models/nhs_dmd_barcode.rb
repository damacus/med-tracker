# frozen_string_literal: true

class NhsDmdBarcode < ApplicationRecord
  has_one :amp_trade_family, primary_key: :amp_code, foreign_key: :amp_code,
                             class_name: 'NhsDmdAmpTradeFamily'

  validates :gtin, presence: true, uniqueness: true
  validates :code, presence: true
  validates :display, presence: true
  validates :system, presence: true

  after_commit :expire_cache

  def self.normalize_gtin(value)
    value.to_s.gsub(/\D/, '')
  end

  def trade_family_metadata
    family = amp_trade_family&.trade_family
    return {} unless family

    family.provenance
  end

  private

  def expire_cache
    NhsDmd::BarcodeLookup.expire(gtin)
  end
end
