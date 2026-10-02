module MedicationPauseContext
  extend ActiveSupport::Concern

  private

  def pause_with_context?(source)
    attributes = params.fetch(:pause_period, ActionController::Parameters.new).permit(:reason, :note)
    unless MedicationPausePeriod::PUBLIC_REASONS.include?(attributes[:reason])
      render_pause_form(source, attributes: attributes, error: t('medication_pauses.choose_reason'),
                                status: :unprocessable_content)
      return false
    end

    source.pause!(reason: attributes[:reason], note: attributes[:note])
    true
  end

  def render_pause_form(source, attributes: {}, error: nil, status: :ok)
    options = { source: source, attributes: attributes, error: error }
    render_modal_or_page(
      modal: -> { Components::MedicationPauses::Form.new(**options, modal: true) },
      page: -> { Components::MedicationPauses::Form.new(**options) },
      status: status
    )
  end
end
