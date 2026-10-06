# frozen_string_literal: true

# Introduces the `people` table and associates existing users and prescriptions.
class CreatePeopleAndReassociateUsers < ActiveRecord::Migration[8.0]
  # Minimal Active Record class for migrating users.
  class MigrationUser < ActiveRecord::Base
    self.table_name = 'users'
  end

  # Minimal Active Record class for migrating people records.
  class MigrationPerson < ActiveRecord::Base
    self.table_name = 'people'
  end

  # Minimal Active Record class for migrating prescriptions.
  class MigrationPrescription < ActiveRecord::Base
    self.table_name = 'prescriptions'
  end

  def up
    create_people_table
    add_person_reference_to_users
    add_person_reference_to_prescriptions
    backfill_people_from_users
    reassign_prescriptions_to_people
    enforce_person_constraints
    remove_user_reference_from_prescriptions
    remove_user_profile_columns
  end

  def down
    restore_user_profile_columns
    restore_user_reference_on_prescriptions
    remove_person_reference_from_users
    drop_people_table
  end

  private

  def create_people_table
    return if table_exists?(:people)

    create_table :people do |t|
      t.string :name, null: false
      t.string :email
      t.date :date_of_birth
      t.timestamps
    end

    add_index :people, :email, unique: true unless index_exists?(:people, :email, unique: true)
  end

  def add_person_reference_to_users
    add_reference :users, :person, foreign_key: true unless column_exists?(:users, :person_id)

    if index_exists?(:users, :person_id) && !index_exists?(:users, :person_id, unique: true)
      remove_index :users, :person_id
    end

    return if index_exists?(:users, :person_id, unique: true)

    add_index :users, :person_id, unique: true
  end

  def add_person_reference_to_prescriptions
    return if column_exists?(:prescriptions, :person_id)

    add_reference :prescriptions, :person, foreign_key: true
  end

  def backfill_people_from_users
    MigrationUser.reset_column_information
    MigrationPerson.reset_column_information

    say_with_time 'Backfilling people for existing users' do
      MigrationUser.where(person_id: nil).find_each do |user|
        person = MigrationPerson.create!(
          name: user[:name] || 'Unknown',
          email: user[:email_address],
          date_of_birth: user[:date_of_birth],
          created_at: user[:created_at] || Time.current,
          updated_at: user[:updated_at] || Time.current
        )

        user.update!(person_id: person.id)
      end
    end
  end

  def reassign_prescriptions_to_people
    MigrationPrescription.reset_column_information

    say_with_time 'Re-associating prescriptions' do
      MigrationPrescription.where(person_id: nil).find_each do |prescription|
        next unless prescription[:user_id]

        user = MigrationUser.find_by(id: prescription[:user_id])
        next unless user&.person_id

        prescription.update!(person_id: user.person_id)
      end
    end
  end

  def enforce_person_constraints
    change_column_null :users, :person_id, false if column_exists?(:users, :person_id)
    change_column_null :prescriptions, :person_id, false if column_exists?(:prescriptions, :person_id)
  end

  def remove_user_reference_from_prescriptions
    return unless column_exists?(:prescriptions, :user_id)

    remove_reference :prescriptions, :user, foreign_key: true
  end

  def remove_user_profile_columns
    remove_column :users, :name if column_exists?(:users, :name)
    remove_column :users, :date_of_birth if column_exists?(:users, :date_of_birth)
  end

  def restore_user_profile_columns
    add_column :users, :name, :string unless column_exists?(:users, :name)
    add_column :users, :date_of_birth, :date unless column_exists?(:users, :date_of_birth)

    MigrationUser.reset_column_information
    MigrationPerson.reset_column_information

    say_with_time 'Restoring users from people' do
      MigrationUser.find_each do |user|
        person = MigrationPerson.find_by(id: user.person_id)
        next unless person

        user.update!(name: person[:name], date_of_birth: person[:date_of_birth])
      end
    end
  end

  def restore_user_reference_on_prescriptions
    add_reference :prescriptions, :user, foreign_key: true unless column_exists?(:prescriptions, :user_id)

    MigrationPrescription.reset_column_information
    MigrationUser.reset_column_information

    say_with_time 'Restoring prescriptions' do
      MigrationPrescription.where(user_id: nil).find_each do |prescription|
        person = MigrationPerson.find_by(id: prescription[:person_id])
        next unless person

        user = MigrationUser.find_by(person_id: person.id)
        next unless user

        prescription.update!(user_id: user.id)
      end
    end

    change_column_null :prescriptions, :user_id, false if column_exists?(:prescriptions, :user_id)
    remove_reference :prescriptions, :person, foreign_key: true if column_exists?(:prescriptions, :person_id)
  end

  def remove_person_reference_from_users
    return unless column_exists?(:users, :person_id)

    change_column_null :users, :person_id, true
    remove_index :users, :person_id if index_exists?(:users, :person_id)
    remove_reference :users, :person, foreign_key: true
  end

  def drop_people_table
    drop_table :people if table_exists?(:people)
  end
end
