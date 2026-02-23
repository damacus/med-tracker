# frozen_string_literal: true

# rubocop:disable Rails/SquishedSQLHeredocs
class BackfillDefaultLocation < ActiveRecord::Migration[8.1]
  def up
    location = execute(<<~SQL).first
      INSERT INTO locations (name, description, created_at, updated_at)
      VALUES ('Home', 'Default home location', NOW(), NOW())
      ON CONFLICT (name) DO UPDATE SET name = 'Home'
      RETURNING id
    SQL

    location_id = location['id']

    execute(<<~SQL)
      UPDATE medicines SET location_id = #{location_id} WHERE location_id IS NULL
    SQL

    execute(<<~SQL)
      INSERT INTO location_memberships (location_id, person_id, created_at, updated_at)
      SELECT DISTINCT #{location_id}, p.id, NOW(), NOW()
      FROM people p
      WHERE p.id IN (
        SELECT DISTINCT person_id FROM prescriptions
        UNION
        SELECT DISTINCT person_id FROM person_medicines
      )
      ON CONFLICT (person_id, location_id) DO NOTHING
    SQL
  end

  def down
    execute(<<~SQL)
      DELETE FROM location_memberships
      WHERE location_id IN (SELECT id FROM locations WHERE name = 'Home')
    SQL

    execute(<<~SQL)
      UPDATE medicines SET location_id = NULL
    SQL

    execute(<<~SQL)
      DELETE FROM locations WHERE name = 'Home'
    SQL
  end
end
# rubocop:enable Rails/SquishedSQLHeredocs
