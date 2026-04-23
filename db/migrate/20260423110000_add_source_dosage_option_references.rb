# frozen_string_literal: true

class AddSourceDosageOptionReferences < ActiveRecord::Migration[8.0]
  def change
    add_reference :schedules, :source_dosage_option, foreign_key: { to_table: :dosages }
    add_reference :person_medications, :source_dosage_option, foreign_key: { to_table: :dosages }
  end
end
