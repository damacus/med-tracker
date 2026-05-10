# frozen_string_literal: true

module GlobalSearch
  class PersonMedicationsResultsQuery < RecordResultsQuery
    def call
      scoped(PersonMedication)
        .joins(:person, :medication)
        .includes(:person, :medication)
        .where(search_sql, term: search_term)
        .order("medications.name ASC", "people.name ASC")
        .limit(limit)
        .map { |person_medication| result_for(person_medication) }
    end

    private

    def result_for(person_medication)
      builder.build(
        type: "person_medication",
        title: person_medication.medication.name,
        subtitle: I18n.t("global_search.subtitles.person_medication", person: person_medication.person.name),
        path: person_path(person_medication.person, anchor: "person_medication_#{person_medication.id}"),
        secondary_values: [person_medication.person.name]
      )
    end

    def search_sql
      "medications.name ILIKE :term OR people.name ILIKE :term"
    end
  end
end
