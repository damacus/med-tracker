# frozen_string_literal: true

class NhsDmdAmpTradeFamily < ApplicationRecord
  belongs_to :trade_family, class_name: 'NhsDmdTradeFamily'

  validates :amp_code, presence: true, uniqueness: true
end
