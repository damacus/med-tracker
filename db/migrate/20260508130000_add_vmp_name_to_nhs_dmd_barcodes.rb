# frozen_string_literal: true

class AddVmpNameToNhsDmdBarcodes < ActiveRecord::Migration[8.1]
  def change
    add_column :nhs_dmd_barcodes, :vmp_name, :string
  end
end
