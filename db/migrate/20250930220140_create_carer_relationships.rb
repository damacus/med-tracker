# frozen_string_literal: true

class CreateCarerRelationships < ActiveRecord::Migration[8.0]
  def change
    drop_table :carer_relationships if table_exists?(:carer_relationships)

    create_table :carer_relationships do |t|
      t.references :carer, null: false, foreign_key: { to_table: :people }
      t.references :patient, null: false, foreign_key: { to_table: :people }
      t.string :relationship_type
      t.boolean :active, default: true, null: false

      t.timestamps
    end

    add_index :carer_relationships, %i[carer_id patient_id], unique: true
    add_index :carer_relationships, :active
  end
end
