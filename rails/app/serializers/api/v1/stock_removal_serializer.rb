module Api
  module V1
    class StockRemovalSerializer
      def initialize(event)
        @event = event
      end

      def as_json(*)
        values = JSON.parse(@event.object)
        values.slice('quantity', 'reason', 'note', 'submission_id',
                     'previous_quantity', 'remaining_quantity', 'unit').merge(
                       id: @event.id.to_s, medication_id: @event.item_id.to_s,
                       dosage_id: values['dosage_id'].presence,
                       created_at: @event.created_at.iso8601, actor_membership_id: @event.actor_membership_id&.to_s
                     )
      end
    end
  end
end
