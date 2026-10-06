# frozen_string_literal: true

module GlobalSearch
  class PeopleResultsQuery < RecordResultsQuery
    def call
      scoped(Person)
        .where('people.name ILIKE ?', search_term)
        .order(:name)
        .limit(limit)
        .map { |person| result_for(person) }
    end

    private

    def result_for(person)
      builder.build(
        type: 'person',
        title: person.name,
        subtitle: I18n.t('global_search.types.person'),
        path: person_path(*tenant_route_args(person))
      )
    end
  end
end
