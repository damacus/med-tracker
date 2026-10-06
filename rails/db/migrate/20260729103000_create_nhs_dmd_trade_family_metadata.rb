# frozen_string_literal: true

class CreateNhsDmdTradeFamilyMetadata < ActiveRecord::Migration[8.1]
  def change
    add_column :nhs_dmd_barcodes, :amp_code, :string
    add_index :nhs_dmd_barcodes, :amp_code

    create_table :nhs_dmd_trade_family_groups do |t|
      t.string :code, null: false
      t.string :name, null: false
      t.timestamps
    end
    add_index :nhs_dmd_trade_family_groups, :code, unique: true

    create_table :nhs_dmd_trade_families do |t|
      t.string :code, null: false
      t.string :name, null: false
      t.references :trade_family_group, foreign_key: { to_table: :nhs_dmd_trade_family_groups }
      t.timestamps
    end
    add_index :nhs_dmd_trade_families, :code, unique: true

    create_table :nhs_dmd_amp_trade_families do |t|
      t.string :amp_code, null: false
      t.references :trade_family, null: false, foreign_key: { to_table: :nhs_dmd_trade_families }
      t.timestamps
    end
    add_index :nhs_dmd_amp_trade_families, :amp_code, unique: true

    create_table :nhs_dmd_supplementary_releases do |t|
      t.date :released_on, null: false
      t.timestamps
    end
    add_index :nhs_dmd_supplementary_releases, :released_on, unique: true
  end
end
