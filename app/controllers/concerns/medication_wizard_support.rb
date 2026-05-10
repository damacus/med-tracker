# frozen_string_literal: true

module MedicationWizardSupport
  extend ActiveSupport::Concern

  private

  def create_success(notice: t("medications.created"))
    return redirect_to(@medication, notice: notice) unless params[:wizard] == "true"

    respond_to do |format|
      format.turbo_stream do
        render(
          turbo_stream: turbo_stream.replace(
            "wizard-content",
            Components::Medications::Wizard::StepDosages.new(medication: @medication)
          )
        )
      end

      format.html { redirect_to(@medication, notice: notice) }
    end
  end

  def wizard_wrapper_class
    case current_user.wizard_variant
    when "modal"
      Components::Medications::Wizard::ModalWrapper
    when "slideover"
      Components::Medications::Wizard::SlideOverWrapper
    else
      Components::Medications::Wizard::FullPageWrapper
    end
  end
end
