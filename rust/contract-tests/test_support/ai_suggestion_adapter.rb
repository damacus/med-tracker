module AiMedication
  class SuggestionService
    def call(medication_identity:, user:)
      raise ArgumentError, 'Unexpected contract medication' unless medication_identity[:name] == 'Calpol Six Plus'
      raise ArgumentError, 'Missing contract user' unless user

      AiMedication::Suggestion.new(
        medication: { description: 'Paracetamol pain and fever relief', warnings: 'Contains paracetamol' },
        doses: [{ amount: 5, unit: 'ml', description: 'Children 6-8 years',
                  default_max_daily_doses: 4, default_min_hours_between_doses: 4,
                  default_dose_cycle: 'daily', evidence: {
                    url: 'https://www.calpol.co.uk/our-products/calpol-sixplus-oral-suspension-paracetamol',
                    title: 'CALPOL SixPlus', text: 'Children 6-8 years 5ml Up to 4 times in 24 hours'
                  } }],
        sources: [{ url: 'https://www.calpol.co.uk/our-products/calpol-sixplus-oral-suspension-paracetamol',
                    title: 'CALPOL SixPlus' }]
      )
    end
  end
end
