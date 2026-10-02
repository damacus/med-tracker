module Api
  module V1
    class StockRemovalsController < BaseController
      rescue_from ActiveRecord::RecordInvalid, with: :render_invalid_removal

      def index
        page = paginate(RemoveMedicationStockService.history(medication).order(id: :desc))
        render json: { data: page[:records].map { |event| StockRemovalSerializer.new(event).as_json }, meta: page[:meta] }
      end

      def create
        reject_numeric_contract_values!(%w[quantity dosage_id])
        attributes = params.expect(stock_removal: %i[quantity reason note dosage_id submission_id]).to_h.symbolize_keys
        return render_invalid_removal unless attributes[:quantity].to_s.match?(/\A[0-9]+(?:\.[0-9]{1,2})?\z/)

        result = RemoveMedicationStockService.new.call(medication: medication, **attributes)
        return render_invalid_removal unless result.success?

        event = RemoveMedicationStockService.find_removal(medication, attributes[:submission_id])
        render json: { data: StockRemovalSerializer.new(event).as_json }, status: :created
      end

      private

      def with_api_idempotency(&)
        authorize medication, :update?
        super
      end

      def medication
        @medication ||= find_api_record(policy_scope(Medication), params.expect(:medication_id))
      end

      def render_invalid_removal
        render_unprocessable('Stock removal could not be recorded')
      end
    end
  end
end
