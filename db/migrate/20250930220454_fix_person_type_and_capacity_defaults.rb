# frozen_string_literal: true

class FixPersonTypeAndCapacityDefaults < ActiveRecord::Migration[8.0]
  def up
    # Update existing records
    execute 'UPDATE people SET person_type = 0 WHERE person_type IS NULL'
    execute 'UPDATE people SET has_capacity = 1 WHERE has_capacity IS NULL'

    # Change column defaults and constraints
    change_column_default :people, :person_type, from: nil, to: 0
    change_column_default :people, :has_capacity, from: nil, to: true
    change_column_null :people, :person_type, false, 0
    change_column_null :people, :has_capacity, false, true
  end

  def down
    change_column_null :people, :person_type, true
    change_column_null :people, :has_capacity, true
    change_column_default :people, :person_type, from: 0, to: nil
    change_column_default :people, :has_capacity, from: true, to: nil
  end
end
