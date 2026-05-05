# frozen_string_literal: true

module PersonMedicationResponseRenderable
  extend ActiveSupport::Concern

  private

  def render_person_medication_create_success
    respond_to do |format|
      format.html { redirect_to person_path(@person), notice: t('person_medications.created') }
      format.turbo_stream do
        flash.now[:notice] = t('person_medications.created')
        render turbo_stream: [
          turbo_stream.update('modal', ''),
          turbo_stream.replace("person_#{@person.id}", Components::People::PersonCard.new(person: @person.reload)),
          turbo_stream.replace("person_show_#{@person.id}", person_show_view(@person.reload)),
          turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
        ]
      end
    end
  end

  def render_person_medication_update_success
    respond_to do |format|
      format.html { redirect_to person_path(@person), notice: t('person_medications.updated') }
      format.turbo_stream do
        flash.now[:notice] = t('person_medications.updated')
        render turbo_stream: [
          turbo_stream.update('modal', ''),
          turbo_stream.replace("person_show_#{@person.id}", person_show_view(@person.reload)),
          turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
        ]
      end
    end
  end

  def render_person_medication_destroy_success
    respond_to do |format|
      format.html { redirect_back_or_to person_path(@person), notice: t('person_medications.deleted') }
      format.turbo_stream do
        flash.now[:notice] = t('person_medications.deleted')
        render turbo_stream: [
          turbo_stream.replace("person_show_#{@person.id}", person_show_view(@person.reload)),
          turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
        ]
      end
    end
  end

  def render_person_medication_reorder_success
    respond_to do |format|
      format.html { redirect_back_or_to person_path(@person) }
      format.turbo_stream do
        render turbo_stream: turbo_stream.replace("person_show_#{@person.id}", person_show_view(@person.reload))
      end
    end
  end
end
