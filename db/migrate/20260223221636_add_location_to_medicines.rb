# frozen_string_literal: true

class AddLocationToMedicines < ActiveRecord::Migration[8.1]
  def change
    add_reference :medicines, :location, null: true, foreign_key: { deferrable: :deferred }
  end
end
