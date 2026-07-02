# frozen_string_literal: true

class AddOrderDetailsToMedications < ActiveRecord::Migration[8.1]
  def change
    add_column :medications, :order_supplier, :string
    add_column :medications, :order_quantity, :decimal, precision: 10, scale: 2
    add_column :medications, :expected_arrival_on, :date
  end
end
