class MakePeopleEmailUniqueOnlyWhenPresent < ActiveRecord::Migration[8.1]
  def up
    execute "UPDATE people SET email = NULL WHERE btrim(email) = ''"

    remove_index :people, name: 'index_people_on_email', if_exists: true
    add_index :people, :email, unique: true, where: "email IS NOT NULL AND btrim(email) <> ''", name: 'index_people_on_email_present_unique'
  end

  def down
    remove_index :people, name: 'index_people_on_email_present_unique', if_exists: true
    add_index :people, :email, unique: true, name: 'index_people_on_email'
  end
end
