class CreateUsers < ActiveRecord::Migration[8.0]
  def change
    create_table :users do |t|
      t.string :email_address, null: false, index: { unique: true }
      t.string :password_digest, null: false

      t.string :name
      t.date :date_of_birth
      t.integer :role, default: 0, null: false

      t.timestamps
    end
  end
end
