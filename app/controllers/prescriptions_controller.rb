class PrescriptionsController < ApplicationController
  before_action :set_person
  before_action :set_prescription, only: [ :edit, :update, :destroy, :take_medicine ]

  def new
    @prescription = @person.prescriptions.build
    @medicines = Medicine.all
  end

  def create
    @prescription = @person.prescriptions.build(prescription_params)

    if @prescription.save
      redirect_to person_path(@person), notice: "Prescription was successfully created."
    else
      @medicines = Medicine.all
      render :new, status: :unprocessable_entity
    end
  end

  def edit
    @medicines = Medicine.all
  end

  def update
    if @prescription.update(prescription_params)
      redirect_to person_path(@person), notice: "Prescription was successfully updated."
    else
      @medicines = Medicine.all
      render :edit, status: :unprocessable_entity
    end
  end

  def destroy
    @prescription.destroy
    redirect_to person_path(@person), notice: "Prescription was successfully deleted."
  end

  def take_medicine
    # Extract the amount from the prescription's dosage if not provided
    amount = params[:amount_ml] || @prescription.dosage.to_f
    
    @take = @prescription.take_medicines.create!(
      taken_at: Time.current,
      amount_ml: amount
    )
    redirect_to person_path(@person), notice: "Medicine taken successfully."
  end

  private

  def set_person
    @person = Person.find(params[:person_id])
  end

  def set_prescription
    @prescription = @person.prescriptions.find(params[:id])
  end

  def prescription_params
    params.require(:prescription).permit(:medicine_id, :dosage, :frequency,
                                       :start_date, :end_date, :notes)
  end
end
