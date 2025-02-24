class PeopleController < ApplicationController
  before_action :set_person, only: [ :show, :update, :destroy ]

  def index
    @people = Person.all
  end

  def show
    @prescriptions = @person.prescriptions.includes(:medicine)
  end

  # GET /people/new
  def new
    @person = Person.new
  end

  # GET /people/:id/edit
  def edit
    @person = Person.find(params[:id])
  end

  def create
    @person = Person.new(person_params)

    respond_to do |format|
      if @person.save
        format.html { redirect_to @person, notice: "Person was successfully created." }
        format.turbo_stream { redirect_to people_path, notice: "Person was successfully created." }
      else
        format.html { render :new, status: :unprocessable_entity }
        format.turbo_stream do
          render turbo_stream: turbo_stream.replace(
            "person_form",
            partial: "form",
            locals: { person: @person }
          )
        end
      end
    end
  end

  def update
    respond_to do |format|
      if @person.update(person_params)
        format.html { redirect_to @person, notice: "Person was successfully updated." }
        format.turbo_stream { redirect_to people_path, notice: "Person was successfully updated." }
        format.json { render :show, status: :ok, location: @person }
      else
        format.html { render :edit, status: :unprocessable_entity }
        format.json { render json: @person.errors, status: :unprocessable_entity }
      end
    end
  end

  # DELETE /people/1 or /people/1.json
  def destroy
    @person.destroy!

    respond_to do |format|
      format.html { redirect_to people_path, status: :see_other, notice: "Person was successfully deleted." }
      format.json { head :no_content }
    end
  end

  private

  def set_person
    @person = Person.find(params[:id])
  end

  def person_params
    params.require(:person).permit(:name, :date_of_birth, :email)
  end
end
