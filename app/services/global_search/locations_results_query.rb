# frozen_string_literal: true

module GlobalSearch
  class LocationsResultsQuery < RecordResultsQuery
    def call
      scoped(Location)
        .where("locations.name ILIKE ?", search_term)
        .order(:name)
        .limit(limit)
        .map { |location| result_for(location) }
    end

    private

    def result_for(location)
      builder.build(
        type: "location",
        title: location.name,
        subtitle: I18n.t("global_search.types.location"),
        path: location_path(location)
      )
    end
  end
end
