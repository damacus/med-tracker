module Api
  module V1
    class MedicationReviewPromptSerializer
      REVIEW_ATTRIBUTES = %w[status practitioner_name practitioner_role reviewed_on review_note
                             reviewed_by_membership_id].freeze
      ID_ATTRIBUTES = %w[id person_id primary_medication_id interacting_medication_id evidence_record_id
                         reviewed_by_membership_id].freeze

      def initialize(prompt)
        @prompt = prompt
      end

      def as_json(*)
        attributes = @prompt.attributes.slice(*(MedicationReviewPrompt::SNAPSHOT_ATTRIBUTES + REVIEW_ATTRIBUTES))
                            .except('household_id')
                            .merge('id' => @prompt.id, 'etag' => Api::RecordEtag.for(@prompt),
                                   'updated_at' => @prompt.updated_at.iso8601)
        ID_ATTRIBUTES.each { |name| attributes[name] = attributes[name]&.to_s }
        attributes
      end
    end
  end
end
