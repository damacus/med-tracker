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
      format.html { render :new, locals: { inline: false, medicines: @medicines } }
      format.turbo_stream do
        render turbo_stream: turbo_stream.replace('modal', partial: 'shared/modal', locals: {
                                                    title: t('prescriptions.modal.new_title', person: @person.name),
                                                    content: render_to_string(partial: 'form',
                                                                              locals: { prescription: @prescription,
                                                                                        inline: true, medicines: @medicines })
                                                  })
      end
    end
  end

  def edit
    authorize @prescription
    @medicines = policy_scope(Medicine)

    respond_to do |format|
      format.html { render :edit, locals: { medicines: @medicines } }
      format.turbo_stream do
        render turbo_stream: turbo_stream.replace('modal', partial: 'shared/modal', locals: {
                                                    title: t('prescriptions.modal.edit_title', person: @person.name),
                                                    content: render_to_string(partial: 'form',
                                                                              locals: { prescription: @prescription,
                                                                                        inline: true, medicines: @medicines })
                                                  })
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
            turbo_stream.remove('modal'),
            turbo_stream.replace("person_#{@person.id}", partial: 'people/person', locals: { person: @person.reload }),
            turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
          ]
        end
      end
    else
      respond_to do |format|
        format.html { render :new, status: :unprocessable_entity, locals: { medicines: @medicines } }
        format.turbo_stream do
          render turbo_stream: turbo_stream.update('modal', partial: 'shared/modal', locals: {
                                                     title: t('prescriptions.modal.new_title', person: @person.name),
                                                     content: render_to_string(partial: 'form',
                                                                               locals: { prescription: @prescription,
                                                                                         inline: true, medicines: @medicines })
                                                   }), status: :unprocessable_entity
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
          render turbo_stream: [
            turbo_stream.update('modal', ''),
            turbo_stream.update('prescriptions',
                                render_to_string(partial: 'prescriptions/prescription',
                                                 collection: @person.reload.prescriptions,
                                                 as: :prescription,
                                                 locals: { person: @person })),
            turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
          ]
        end
      end
    else
      @medicines = Medicine.all
      respond_to do |format|
        format.html { render :edit, status: :unprocessable_entity, locals: { medicines: @medicines } }
        format.turbo_stream do
          render turbo_stream: turbo_stream.update('modal', partial: 'shared/modal', locals: {
                                                     title: t('prescriptions.modal.edit_title', person: @person.name),
                                                     content: render_to_string(partial: 'form',
                                                                               locals: { prescription: @prescription,
                                                                                         inline: true, medicines: @medicines })
                                                   }), status: :unprocessable_entity
        end
      end
    end
  end

  def destroy
    authorize @prescription
    @prescription.destroy
    redirect_back fallback_location: person_path(@person), notice: t('prescriptions.deleted')
  end

  def take_medicine
    authorize @prescription, :take_medicine?
    # Extract the amount from the prescription's dosage if not provided
    amount = params[:amount_ml] || @prescription.dosage.amount

    @take = @prescription.take_medicines.create!(
      taken_at: Time.current,
      amount_ml: amount
    )
    redirect_back fallback_location: person_path(@person), notice: t('prescriptions.medicine_taken')
  end

  private

  def set_person
    @person = Person.find(params[:person_id])
  end

  def set_prescription
    @prescription = policy_scope(Prescription).find(params[:id])
  end

  def prescription_params
    params.expect(prescription: %i[medicine_id dosage frequency
                                   start_date end_date notes])
  end
end
