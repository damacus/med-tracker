# frozen_string_literal: true

class CreateLocationMemberships < ActiveRecord::Migration[8.1]
  def change
    create_table :location_memberships do |t|
      t.references :location, null: false, foreign_key: { deferrable: :deferred }
      t.references :person, null: false, foreign_key: { deferrable: :deferred }

      t.timestamps
    end

    add_index :location_memberships, %i[person_id location_id], unique: true
  end
end
