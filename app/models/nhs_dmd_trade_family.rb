# frozen_string_literal: true

class NhsDmdTradeFamily < ApplicationRecord
  belongs_to :trade_family_group, class_name: 'NhsDmdTradeFamilyGroup', optional: true
  has_many :amp_trade_families, class_name: 'NhsDmdAmpTradeFamily', dependent: :restrict_with_exception

  validates :code, :name, presence: true
  validates :code, uniqueness: true
end
