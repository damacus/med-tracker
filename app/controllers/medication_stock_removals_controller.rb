class MedicationStockRemovalsController < ApplicationController
  before_action :set_medication

  def new
    render_form(submission_id: SecureRandom.uuid)
  end

  def create
    attributes = params.expect(stock_removal: %i[quantity reason note dosage_id submission_id]).to_h.symbolize_keys
    result = RemoveMedicationStockService.new.call(medication: @medication, **attributes)
    return render_form(attributes, error: result.error, status: :unprocessable_content) unless result.success?

    redirect_to medication_path(@medication), notice: t('stock_removals.success'), status: :see_other
  end

  private

  def set_medication
    @medication = policy_scope(Medication).find_by(id: params[:medication_id])
    return head :not_found unless @medication

    authorize @medication, :update?
  end

  def render_form(attributes = {}, error: nil, status: :ok, **initial_attributes)
    removals = RemoveMedicationStockService.history(@medication).order(id: :desc).limit(20).to_a
    actor_names = User.joins(:person).where(id: removals.map(&:whodunnit)).pluck(:id, 'people.name').to_h
    render Components::Medications::StockRemovalView.new(
      medication: @medication,
      dosages: @medication.dosage_records.where.not(current_supply: nil).order(:id).to_a,
      removals: removals.map { |event| JSON.parse(event.object).merge('at' => event.created_at, 'actor' => actor_names[event.whodunnit.to_i]) },
      attributes: attributes.merge(initial_attributes), error: error
    ), status: status
  end
end
