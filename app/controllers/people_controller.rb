# frozen_string_literal: true

class PeopleController < ApplicationController
  include PersonViewable

  before_action :set_person, only: %i[show update destroy add_medication]

  def index
    authorize Person
    people = PeopleIndexQuery.new(scope: policy_scope(Person)).call
    render Components::People::IndexView.new(people: people)
  end

  def show
    authorize @person
    show_data = PersonShowQuery.new(person: @person).call

    render Components::People::ShowView.new(
      person: @person,
      schedules: show_data.schedules,
      person_medications: show_data.person_medications,
      current_user: current_user
    )
  end

  def new
    @person = Person.new
    authorize @person
    is_modal = request.headers['Turbo-Frame'] == 'modal'
    assigned_location = primary_location

    if is_modal
      render Components::People::Modal.new(person: @person, assigned_location: assigned_location), layout: false
    else
      render Components::People::FormView.new(person: @person, assigned_location: assigned_location)
    end
  end

  def edit
    @person = policy_scope(Person).find(params.expect(:id))
    authorize @person
    @return_to = url_from(params[:return_to])
    is_modal = request.headers['Turbo-Frame'] == 'modal'

    if is_modal
      render Components::People::Modal.new(person: @person, return_to: @return_to), layout: false
    else
      render Components::People::FormView.new(person: @person, return_to: @return_to)
    end
  end

  def create
    @person = Person.new(person_params)
    @person.household = current_household if current_household
    authorize @person
    @person.primary_location = primary_location

    if auto_assign_created_person_carer_relationship?
      @person.carer_relationships.build(
        carer: current_membership.person,
        relationship_type: :family_member,
        active: true
      )
    end

    respond_to do |format|
      if @person.save
        grant_created_person_access
        format.html { redirect_to @person, notice: t('people.created') }
        format.turbo_stream do
          flash.now[:notice] = t('people.created')
          render turbo_stream: [
            turbo_stream.update('modal', ''),
            turbo_stream.prepend(tenant_dom_target('people'), Components::People::PersonCard.new(person: @person.reload)),
            turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
          ]
        end
      else
        format.html do
          render Components::People::FormView.new(person: @person, assigned_location: primary_location),
                 status: :unprocessable_content
        end
        format.turbo_stream do
          render turbo_stream: turbo_stream.replace(
            'modal',
            Components::People::Modal.new(person: @person, assigned_location: primary_location)
          ), status: :unprocessable_content
        end
      end
    end
  end

  def update
    authorize @person
    respond_to do |format|
      if @person.update(person_params)
        format.html { redirect_to safe_redirect_path(params[:return_to]) || @person, notice: t('people.updated') }
        format.turbo_stream do
          flash.now[:notice] = t('people.updated')
          render turbo_stream: [
            turbo_stream.update('modal', ''),
            turbo_stream.replace(tenant_dom_id(@person), Components::People::PersonCard.new(person: @person.reload)),
            turbo_stream.replace(tenant_dom_target("person_show_#{@person.id}"), person_show_view(@person.reload)),
            turbo_stream.update('flash', Components::Layouts::Flash.new(notice: flash[:notice], alert: flash[:alert]))
          ]
        end
        format.json { render :show, status: :ok, location: @person }
      else
        format.html do
          render Components::People::FormView.new(person: @person, return_to: url_from(params[:return_to])), status: :unprocessable_content
        end
        format.turbo_stream do
          render turbo_stream: turbo_stream.replace(
            'modal',
            Components::People::Modal.new(person: @person, return_to: url_from(params[:return_to]))
          ), status: :unprocessable_content
        end
        format.json { render json: @person.errors, status: :unprocessable_content }
      end
    end
  end

  # DELETE /people/1 or /people/1.json
  def destroy
    authorize @person
    @person.destroy!

    respond_to do |format|
      format.html { redirect_to people_path, status: :see_other, notice: t('people.deleted') }
      format.json { head :no_content }
    end
  end

  def add_medication
    authorize @person, :show?
    redirect_to new_person_medication_assignment_path(
      @person,
      source: params[:source],
      medication_id: params[:medication_id]
    )
  end

  private

  def set_person
    @person = policy_scope(Person).find(params.expect(:id))
  end

  def person_params
    params.expect(person: %i[name date_of_birth email person_type has_capacity])
  end

  def primary_location
    PrimaryLocationQuery.new(person: current_user&.person).call
  end

  def grant_created_person_access
    return unless current_household && current_membership

    current_household.person_access_grants.find_or_create_by!(
      household_membership: current_membership,
      person: @person
    ) do |grant|
      grant.access_level = :manage
      grant.relationship_type = :family_member
      grant.granted_by_membership = current_membership
    end
  end

  def auto_assign_created_person_carer_relationship?
    current_membership&.person.present? && (@person.minor? || @person.dependent_adult?)
  end
end
