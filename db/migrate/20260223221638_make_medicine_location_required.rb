# frozen_string_literal: true

class MakeMedicineLocationRequired < ActiveRecord::Migration[8.1]
  def change
    change_column_null :medicines, :location_id, false
  end
end
