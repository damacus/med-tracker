class AddDurableArchiveReferenceToNhsDmdImports < ActiveRecord::Migration[8.1]
  def change
    change_table :nhs_dmd_imports, bulk: true do |t|
      t.string :archive_service_name
      t.string :archive_key
      t.string :archive_checksum
      t.bigint :archive_byte_size
    end
  end
end
