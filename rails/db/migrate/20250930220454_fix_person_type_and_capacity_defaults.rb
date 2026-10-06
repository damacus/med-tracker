# frozen_string_literal: true

class FixPersonTypeAndCapacityDefaults < ActiveRecord::Migration[8.0]
  def up
    # Update existing records
    execute 'UPDATE people SET person_type = 0 WHERE person_type IS NULL'
    execute 'UPDATE people SET has_capacity = TRUE WHERE has_capacity IS NULL'

    # Change column defaults and constraints
    change_table :people, bulk: true do |t|
      t.change_default :person_type, 0
      t.change_default :has_capacity, true
      t.change_null :person_type, false, 0
      t.change_null :has_capacity, false, true
    end
  end

  def down
    change_table :people, bulk: true do |t|
      t.change_default :person_type, nil
      t.change_default :has_capacity, nil
      t.change_null :person_type, true
      t.change_null :has_capacity, true
    end
  end
end
