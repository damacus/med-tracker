class PeopleController < ApplicationController
  before_action :set_person, only: [ :show, :update, :destroy ]

  def index
    @people = Person.all
  end

  def show
    @prescriptions = @person.prescriptions.includes(:medicine)
  end

  def new
    @person = Person.new
    respond_to do |format|
      format.html
      format.turbo_stream do
        render turbo_stream: turbo_stream.update("modal", partial: "shared/modal", locals: {
          title: "Add Person",
          content: render_to_string(partial: "form", locals: { person: @person })
        })
      end
    end
  end

  def create
    @person = Person.new(person_params)

    if @person.save
      respond_to do |format|
        format.turbo_stream do
          flash.now[:notice] = "Person was successfully created."
          render turbo_stream: [
            turbo_stream.remove("modal"),
            turbo_stream.append("people", partial: "person", locals: { person: @person }),
            turbo_stream.update("flash", partial: "shared/flash")
          ]
        end
        format.html { redirect_to @person, notice: "Person was successfully created." }
      end
    else
      respond_to do |format|
        format.turbo_stream do
          render turbo_stream: turbo_stream.update("modal", partial: "shared/modal", locals: {
            title: "Add Person",
            content: render_to_string(partial: "form", locals: { person: @person })
          })
        end
        format.html { render :new, status: :unprocessable_entity }
      end
    end
  end

  def update
    if @person.update(person_params)
      respond_to do |format|
        format.turbo_stream {
          render turbo_stream: turbo_stream.replace(
            @person,
            partial: "person_details",
            locals: { person: @person, editing: false }
          )
        }
        format.html { redirect_to @person, notice: "Person was successfully updated." }
      end
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
