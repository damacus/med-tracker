class PeopleController < ApplicationController
  before_action :set_person, only: [ :show, :edit, :update, :destroy ]

  def index
    @people = Person.all
  end

  def show
    @prescriptions = @person.prescriptions.includes(:medicine)
  end

  def new
    @person = Person.new
  end

  def edit
  end

  def create
    @person = Person.new(person_params)

    if @person.save
      redirect_to @person, notice: "Person was successfully created."
    else
      render :new, status: :unprocessable_entity
    end
  end

  def update
    if @person.update(person_params)
      redirect_to @person, notice: "Person was successfully updated."
    else
      render :edit, status: :unprocessable_entity
    end
  end

  def destroy
    @person.destroy
    redirect_to people_url, notice: "Person was successfully deleted."
  end

  private

  def set_person
    @person = Person.find(params[:id])
  end

  def person_params
    params.require(:person).permit(:name, :date_of_birth)
  end
end
