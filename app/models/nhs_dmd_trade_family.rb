# frozen_string_literal: true

class NhsDmdTradeFamily < ApplicationRecord
  belongs_to :trade_family_group, class_name: 'NhsDmdTradeFamilyGroup', optional: true
  has_many :amp_trade_families, class_name: 'NhsDmdAmpTradeFamily', dependent: :restrict_with_exception

  validates :code, :name, presence: true
  validates :code, uniqueness: true

  def provenance
    metadata = { trade_family: { code: code, name: name } }
    metadata[:trade_family_group] = { code: trade_family_group.code, name: trade_family_group.name } if trade_family_group
    metadata
  end
end
