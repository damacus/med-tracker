# frozen_string_literal: true

class PrescriptionsController < ApplicationController
  include Pundit::Authorization

  before_action :set_person
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

    # SECURITY: Enforce timing restrictions server-side
    # This prevents bypassing UI-disabled buttons via direct API calls
    unless @prescription.can_administer?
      reason = @prescription.administration_blocked_reason
      message = reason == :out_of_stock ? 'Cannot take medicine: out of stock' : 'Cannot take medicine: timing restrictions not met'
      respond_to do |format|
        format.html do
          redirect_back_or_to person_path(@person),
                              alert: t('prescriptions.cannot_take_medicine', default: message)
        end
        format.turbo_stream do
          flash.now[:alert] = t('prescriptions.cannot_take_medicine', default: message)
          render turbo_stream: turbo_stream.update('flash',
                                                   Components::Layouts::Flash.new(alert: flash[:alert]))
        end
      end
      return
    end

    # Extract the amount from the prescription's dosage if not provided
    amount = params[:amount_ml] || @prescription.dosage.amount

    @take = @prescription.medication_takes.create!(
      taken_at: Time.current,
      amount_ml: amount
    )
    flash.now[:notice] = t('prescriptions.medicine_taken')

    respond_to do |format|
      format.html { redirect_to dashboard_path }
      format.turbo_stream do
        render turbo_stream: [
          turbo_stream.replace("dose_prescription_#{@prescription.id}",
                               Components::Dashboard::TimelineItem.new(dose: {
                                                                         person: @person,
                                                                         source: @prescription.reload,
                                                                         scheduled_at: @take.taken_at,
                                                                         taken_at: @take.taken_at,
                                                                         status: :taken
                                                                       })),
          turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice]))
        ]
      end
    end
  end

  private

  def set_person
    @person = Person.find(params[:person_id])
  end

  def set_prescription
    @prescription = policy_scope(Prescription).find(params[:id])
  end

  def prescription_params
    params.expect(prescription: %i[medicine_id dosage_id frequency
                                   start_date end_date notes max_daily_doses
                                   min_hours_between_doses dose_cycle])
  end
end
