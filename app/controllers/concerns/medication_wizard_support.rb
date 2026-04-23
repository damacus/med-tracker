# frozen_string_literal: true

module MedicationWizardSupport
  extend ActiveSupport::Concern

  private

  def create_success
    return redirect_to(@medication, notice: t('medications.created')) unless params[:wizard] == 'true'

    respond_to do |format|
      format.turbo_stream do
        render turbo_stream: turbo_stream.replace(
          'wizard-content',
          Components::Medications::Wizard::StepDosages.new(medication: @medication)
        )
      end
      format.html { redirect_to @medication, notice: t('medications.created') }
    end
  end

  def wizard_wrapper_class
    case current_user.wizard_variant
    when 'modal'     then Components::Medications::Wizard::ModalWrapper
    when 'slideover' then Components::Medications::Wizard::SlideOverWrapper
    else                  Components::Medications::Wizard::FullPageWrapper
    end
  end
end
