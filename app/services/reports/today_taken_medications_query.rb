# frozen_string_literal: true

module Reports
  class TodayTakenMedicationsQuery
    PersonGroup = Data.define(:person, :medications)
    MedicationSummary = Data.define(:id, :name)

    attr_reader :people

    def initialize(people:)
      @people = people
    end

    def call
      people_by_id.values_at(*person_ids_with_takes).compact.map do |person|
        PersonGroup.new(person: person, medications: medications_for(person.id))
      end
    end

    private

    def person_ids
      @person_ids ||= if people.respond_to?(:pluck)
                        people.pluck(:id)
                      else
                        Array(people).map(&:id)
                      end
    end

    def people_by_id
      @people_by_id ||= Person.where(id: person_ids).order(:name, :id).index_by(&:id)
    end

    def person_ids_with_takes
      @person_ids_with_takes ||= medication_rows_by_person.keys.sort_by { |id| [people_by_id.fetch(id).name, id] }
    end

    def medications_for(person_id)
      medication_rows_by_person.fetch(person_id, []).uniq { |row| row.fetch(:id) }.map do |row|
        MedicationSummary.new(id: row.fetch(:id), name: row.fetch(:name))
      end
    end

    def medication_rows_by_person
      @medication_rows_by_person ||= begin
        rows = schedule_medication_rows + person_medication_rows
        rows.group_by { |row| row.fetch(:person_id) }.transform_values do |person_rows|
          person_rows.sort_by { |row| [row.fetch(:name), row.fetch(:id)] }
        end
      end
    end

    def schedule_medication_rows
      MedicationTake.joins(schedule: :medication)
                    .where(schedules: { person_id: person_ids })
                    .where(taken_at: Time.zone.today.all_day)
                    .distinct
                    .pluck(
                      'schedules.person_id',
                      'medications.id',
                      Arel.sql('COALESCE(NULLIF(medications.friendly_name, \'\'), medications.name)')
                    )
                    .map do |person_id, medication_id, medication_name|
                      medication_row(person_id, medication_id, medication_name)
                    end
    end

    def person_medication_rows
      MedicationTake.joins(person_medication: :medication)
                    .where(person_medications: { person_id: person_ids })
                    .where(taken_at: Time.zone.today.all_day)
                    .distinct
                    .pluck(
                      'person_medications.person_id',
                      'medications.id',
                      Arel.sql('COALESCE(NULLIF(medications.friendly_name, \'\'), medications.name)')
                    )
                    .map do |person_id, medication_id, medication_name|
                      medication_row(person_id, medication_id, medication_name)
                    end
    end

    def medication_row(person_id, medication_id, medication_name)
      { person_id: person_id, id: medication_id, name: medication_name }
    end
  end
end
