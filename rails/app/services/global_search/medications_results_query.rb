# frozen_string_literal: true

module GlobalSearch
  class MedicationsResultsQuery < RecordResultsQuery
    def call
      scoped(Medication)
        .includes(:location)
        .where(search_sql, term: search_term)
        .order(:name)
        .limit(limit)
        .map { |medication| result_for(medication) }
    end

    private

    def result_for(medication)
      builder.build(
        type: 'medication',
        title: medication.display_name,
        subtitle: subtitle_for(medication),
        path: medication_path(*tenant_route_args(medication)),
        secondary_values: [medication.name, medication.category, medication.barcode, medication.dmd_code]
      )
    end

    def subtitle_for(medication)
      [medication.category.presence, medication.location&.name].compact.join(' · ').presence ||
        I18n.t('global_search.types.medication')
    end

    def search_sql
      'medications.name ILIKE :term OR medications.category ILIKE :term OR ' \
        'medications.barcode ILIKE :term OR medications.dmd_code ILIKE :term'
    end
  end
end
