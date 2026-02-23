# frozen_string_literal: true

class AddPositionToPersonMedicines < ActiveRecord::Migration[8.1]
  def up
    add_column :person_medicines, :position, :integer

    execute <<~SQL.squish
      WITH ranked AS (
        SELECT id,
               ROW_NUMBER() OVER (PARTITION BY person_id ORDER BY created_at ASC, id ASC) - 1 AS new_position
        FROM person_medicines
      )
      UPDATE person_medicines
      SET position = ranked.new_position
      FROM ranked
      WHERE person_medicines.id = ranked.id
    SQL

    change_column_null :person_medicines, :position, false
    add_index :person_medicines, %i[person_id position], name: 'index_person_medicines_on_person_id_and_position'
  end

  def down
    remove_index :person_medicines, name: 'index_person_medicines_on_person_id_and_position'
    remove_column :person_medicines, :position
  end
end
