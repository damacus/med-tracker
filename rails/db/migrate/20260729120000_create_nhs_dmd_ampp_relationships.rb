class CreateNhsDmdAmppRelationships < ActiveRecord::Migration[8.1]
  def change
    create_table :nhs_dmd_ampp_relationships do |t|
      t.string :ampp_code, null: false
      t.string :amp_code, null: false
      t.timestamps
    end

    add_index :nhs_dmd_ampp_relationships, :ampp_code, unique: true
    add_index :nhs_dmd_ampp_relationships, :amp_code
  end
end
