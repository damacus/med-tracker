# frozen_string_literal: true

class PrescriptionsController < ApplicationController
  include Pundit::Authorization
  include PersonScoped
  include TakeMedicineResponder

  before_action :set_prescription, only: %i[edit update destroy take_medicine]

  def new
    @prescription = @person.prescriptions.build
    authorize @prescription
    @medicines = policy_scope(Medicine)

    respond_to do |format|
      format.html do
        render Components::Prescriptions::NewView.new(
          prescription: @prescription,
          person: @person,
          medicines: @medicines
        )
      end
      format.turbo_stream do
        render turbo_stream: turbo_stream.replace(
          'prescription_modal',
          Components::Prescriptions::Modal.new(
            prescription: @prescription,
            person: @person,
            medicines: @medicines,
            title: t('prescriptions.modal.new_title', person: @person.name)
          )
        )
      end
    end
  end

  def edit
    authorize @prescription
    @medicines = policy_scope(Medicine)

    respond_to do |format|
      format.html do
        render Components::Prescriptions::EditView.new(
          prescription: @prescription,
          person: @person,
          medicines: @medicines
        )
      end
      format.turbo_stream do
        render turbo_stream: turbo_stream.replace(
          'prescription_modal',
          Components::Prescriptions::Modal.new(
            prescription: @prescription,
            person: @person,
            medicines: @medicines,
            title: t('prescriptions.modal.edit_title', person: @person.name)
          )
        )
      end
    end
  end

  def create
    @prescription = @person.prescriptions.build(prescription_params)
    authorize @prescription
    @medicines = policy_scope(Medicine)

    if @prescription.save
      respond_to do |format|
        format.html { redirect_to person_path(@person), notice: t('prescriptions.created') }
        format.turbo_stream do
          flash.now[:notice] = t('prescriptions.created')
          render turbo_stream: [
            turbo_stream.remove('prescription_modal'),
            turbo_stream.replace("person_#{@person.id}", Components::People::PersonCard.new(person: @person.reload)),
            turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
          ]
        end
      end
    else
      respond_to do |format|
        format.html do
          render Components::Prescriptions::NewView.new(
            prescription: @prescription,
            person: @person,
            medicines: @medicines
          ), status: :unprocessable_content
        end
        format.turbo_stream do
          render turbo_stream: turbo_stream.update(
            'prescription_modal',
            Components::Prescriptions::Modal.new(
              prescription: @prescription,
              person: @person,
              medicines: @medicines,
              title: t('prescriptions.modal.new_title', person: @person.name)
            )
          ), status: :unprocessable_content
        end
      end
    end
  end

  def update
    authorize @prescription
    if @prescription.update(prescription_params)
      respond_to do |format|
        format.html { redirect_to person_path(@person), notice: t('prescriptions.updated') }
        format.turbo_stream do
          flash.now[:notice] = t('prescriptions.updated')
          prescriptions_html = @person.reload.prescriptions.map do |prescription|
            view_context.render(Components::Prescriptions::Card.new(prescription: prescription, person: @person))
          end.join
          render turbo_stream: [
            turbo_stream.update('prescription_modal', ''),
            turbo_stream.update('prescriptions', prescriptions_html),
            turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
          ]
        end
      end
    else
      @medicines = policy_scope(Medicine)
      respond_to do |format|
        format.html do
          render Components::Prescriptions::EditView.new(
            prescription: @prescription,
            person: @person,
            medicines: @medicines
          ), status: :unprocessable_content
        end
        format.turbo_stream do
          render turbo_stream: turbo_stream.update(
            'prescription_modal',
            Components::Prescriptions::Modal.new(
              prescription: @prescription,
              person: @person,
              medicines: @medicines,
              title: t('prescriptions.modal.edit_title', person: @person.name)
            )
          ), status: :unprocessable_content
        end
      end
    end
  end

  def destroy
    authorize @prescription
    @prescription.destroy
    redirect_back_or_to person_path(@person), notice: t('prescriptions.deleted')
  end

  def take_medicine
    authorize @prescription, :take_medicine?
    handle_take_medicine(takeable: @prescription, i18n_scope: 'prescriptions')
  end

  def take_success_turbo_streams(takeable)
    [
      turbo_stream.replace("prescription_#{takeable.id}",
                           Components::Prescriptions::Card.new(prescription: takeable.reload, person: @person)),
      turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice]))
    ]
  end

  private

  def set_prescription
    @prescription = policy_scope(Prescription).find(params[:id])
  end

  def prescription_params
    params.expect(prescription: %i[medicine_id dosage_id frequency
                                   start_date end_date notes max_daily_doses
                                   min_hours_between_doses dose_cycle])
  end
end
