# frozen_string_literal: true

require 'rails_helper'

RSpec.describe 'People' do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :carer_relationships

  describe 'GET /people/new' do
    context 'when signed in as a parent' do
      before { sign_in(users(:jane)) }

      it 'renders household person types' do
        get new_person_path

        expect(response).to have_http_status(:ok)
        expect(response.body).to include('value="adult"')
        expect(response.body).to include('value="minor"')
        expect(response.body).to include('value="dependent_adult"')
      end

      it 'shows the primary location where the dependent will be created' do
        get new_person_path

        expect(response).to have_http_status(:ok)
        expect(response.body).to include('This person will be created at Home.')
      end
    end

    context 'when signed in as an admin' do
      before { sign_in(users(:admin)) }

      it 'renders the household person form' do
        get new_person_path

        expect(response).to have_http_status(:ok)
      end
    end
  end

  describe 'GET /households/:household_slug/people' do
    it 'renders only people granted to the current membership' do
      user = users(:jane)
      household = ensure_api_household_for(user)
      membership = household.household_memberships.find_by!(account: user.person.account)
      visible_person = create(:person, household: household, name: 'Visible Web Alex')
      hidden_person = create(:person, household: household, name: 'Hidden Web Alex')
      household.person_access_grants.create!(
        household_membership: membership,
        person: visible_person,
        access_level: :view,
        relationship_type: :family_member,
        granted_by_membership: membership
      )
      sign_in(user)

      allow(TenantContext).to receive(:with).and_call_original
      allow(TenantContext).to receive(:set_membership!).and_call_original

      get "/households/#{household.slug}/people"

      expect(TenantContext).to have_received(:with).with(
        account: user.person.account,
        household: household,
        request_id: kind_of(String)
      )
      expect(TenantContext).to have_received(:set_membership!).with(membership)
      expect(response).to have_http_status(:ok)
      expect(response.body).to include(visible_person.name)
      expect(response.body).not_to include(hidden_person.name)
    end
  end

  describe 'PATCH /households/:household_slug/people/:id' do
    it 'partitions PaperTrail versions by household and actor membership' do
      user = users(:jane)
      household = ensure_api_household_for(user)
      membership = household.household_memberships.find_by!(account: user.person.account)
      membership.update!(role: :owner, status: :active)
      target = create(:person, household: household, name: 'Versioned Person')
      household.person_access_grants.create!(
        household_membership: membership,
        person: target,
        access_level: :manage,
        relationship_type: :family_member,
        granted_by_membership: membership
      )
      sign_in(user)

      patch "/households/#{household.slug}/people/#{target.id}",
            params: { person: { name: 'Versioned Person Updated' } }

      version = PaperTrail::Version.where(item_type: 'Person', item_id: target.id, event: 'update').last
      expect(version.household_id).to eq(household.id)
      expect(version.actor_membership_id).to eq(membership.id)
    end
  end

  describe 'POST /people' do
    context 'when signed in as a parent' do
      let(:parent_user) { users(:jane) }

      before { sign_in(parent_user) }

      it 'creates a minor and auto-links carer relationship' do
        expect do
          post people_path, params: {
            person: {
              name: 'New Child',
              date_of_birth: 5.years.ago.to_date,
              person_type: 'minor'
            }
          }
        end.to change(Person, :count).by(1).and change(CarerRelationship, :count).by(1)

        created_person = Person.last
        expect(created_person.name).to eq('New Child')
        expect(created_person.household.slug).to eq(default_url_options.fetch(:household_slug))
        expect(created_person.has_capacity).to be false
        expect(created_person.user).to be_nil

        relationship = created_person.carer_relationships.first
        expect(relationship.carer).to eq(parent_user.person)
        expect(relationship.relationship_type).to eq('family_member')
        expect(relationship.active).to be true
        grant = created_person.person_access_grants.find_by(household_membership: current_membership)

        expect(grant.access_level).to eq('manage')
      end

      it 'creates a dependent adult and auto-links carer relationship' do
        expect do
          post people_path, params: {
            person: {
              name: 'Dependent Adult',
              date_of_birth: 70.years.ago.to_date,
              person_type: 'dependent_adult'
            }
          }
        end.to change(Person, :count).by(1)

        created_person = Person.last
        expect(created_person.has_capacity).to be false
        expect(created_person.carer_relationships.first.carer).to eq(parent_user.person)
      end

      it 'allows creating multiple dependents with blank email addresses' do
        expect do
          post people_path, params: {
            person: {
              name: 'First Child',
              date_of_birth: 7.years.ago.to_date,
              person_type: 'minor',
              email: ''
            }
          }

          post people_path, params: {
            person: {
              name: 'Second Child',
              date_of_birth: 8.years.ago.to_date,
              person_type: 'minor',
              email: ''
            }
          }
        end.to change(Person, :count).by(2)

        created_people = Person.order(:id).last(2)
        expect(created_people.map(&:email)).to all(be_blank)
      end

      it 'persists the parent primary location as dependent metadata' do
        parent_user.person.location_memberships.delete_all
        parent_user.person.location_memberships.create!(location: locations(:school))

        post people_path, params: {
          person: {
            name: 'Location Child',
            date_of_birth: 6.years.ago.to_date,
            person_type: 'minor',
            email: ''
          }
        }

        created_person = Person.order(:id).last
        expect(created_person.locations).to contain_exactly(locations(:school))
      end

      it 'creates an adult person in the household' do
        expect do
          post people_path, params: {
            person: {
              name: 'Another Adult',
              date_of_birth: 30.years.ago.to_date,
              person_type: 'adult'
            }
          }
        end.to change(Person, :count).by(1)

        expect(Person.last.household.slug).to eq(default_url_options.fetch(:household_slug))
      end
    end

    context 'when signed in as an admin' do
      before { sign_in(users(:admin)) }

      it 'creates household people' do
        expect do
          post people_path, params: {
            person: {
              name: 'Someone',
              date_of_birth: 10.years.ago.to_date,
              person_type: 'minor'
            }
          }
        end.to change(Person, :count).by(1)

        expect(Person.last.household.slug).to eq(default_url_options.fetch(:household_slug))
      end

      it 'rejects a dependent adult type for a person under 18' do
        expect do
          post people_path, params: {
            person: {
              name: 'Invalid Dependent Adult',
              date_of_birth: 12.years.ago.to_date,
              person_type: 'dependent_adult'
            }
          }
        end.not_to change(Person, :count)

        expect(response).to have_http_status(:unprocessable_content)
        expect(response.body).to include('must be minor or adult for people under 18')
      end
    end
  end

  describe 'POST /people with turbo_stream format' do
    before { sign_in(users(:jane)) }

    it 'returns turbo_stream and updates modal, people list, and flash on success' do
      post people_path,
           params: {
             person: {
               name: 'Turbo Child',
               date_of_birth: 6.years.ago.to_date,
               person_type: 'minor'
             }
           },
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include('target="modal"')
      expect(response.body).to include(%(target="#{household_target('people')}"))
      expect(response.body).to include('target="flash"')
    end

    it 'returns unprocessable content and re-renders modal on failure' do
      post people_path,
           params: {
             person: {
               name: '',
               date_of_birth: '',
               person_type: 'minor'
             }
           },
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include('target="modal"')
      expect(response.body).to include('person_form')
      expect(response.body).to include('role="alert"')
      expect(response.body).to include('id="person_name_error"')
      expect(response.body).to include('aria-describedby="person_name_error"')
      expect(response.body).to include('aria-invalid')
    end

    it 'allows creating multiple dependents in sequence without email addresses' do
      post people_path,
           params: {
             person: {
               name: 'Turbo Child One',
               date_of_birth: 6.years.ago.to_date,
               person_type: 'minor',
               email: ''
             }
           },
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.body).to include(%(target="#{household_target('people')}"))
      expect(response.body).to include('Turbo Child One')

      post people_path,
           params: {
             person: {
               name: 'Turbo Child Two',
               date_of_birth: 5.years.ago.to_date,
               person_type: 'minor',
               email: ''
             }
           },
           headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.body).to include(%(target="#{household_target('people')}"))
      expect(response.body).to include('Turbo Child Two')
    end
  end

  describe 'PATCH /people/:id with turbo_stream format' do
    before { sign_in(users(:admin)) }

    it 'returns turbo_stream and updates card, show container, and flash on success' do
      person = people(:john)

      patch person_path(person),
            params: { person: { name: 'Turbo Updated Name' } },
            headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:ok)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include(%(target="#{household_target("person_#{person.id}")}"))
      expect(response.body).to include(%(target="#{household_target("person_show_#{person.id}")}"))
      expect(response.body).to include('target="flash"')
      expect(person.reload.name).to eq('Turbo Updated Name')
    end

    it 'returns unprocessable content and re-renders modal on failure' do
      person = people(:john)

      patch person_path(person),
            params: { person: { name: '', date_of_birth: '' } },
            headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(response.body).to include('target="modal"')
      expect(response.body).to include('person_form')
    end
  end

  describe 'PATCH /people/:id person type transitions' do
    before { sign_in(users(:admin)) }

    it 'requires a minor who has reached 18 to transition type' do
      person = people(:child_patient)

      patch person_path(person), params: {
        person: {
          date_of_birth: 18.years.ago.to_date,
          person_type: 'minor'
        }
      }

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.body).to include('must be adult or dependent adult for people aged 18 or over')
      expect(person.reload.person_type).to eq('minor')
    end
  end

  describe 'DELETE /people/:id' do
    before { sign_in(users(:admin)) }

    it 'destroys the person and redirects to the people index' do
      person = create_managed_person_for(users(:admin), 'Delete Me')

      expect do
        delete person_path(person)
      end.to change(Person, :count).by(-1)

      expect(response).to redirect_to(people_path)
      expect(flash[:notice]).to eq(I18n.t('people.deleted'))
    end

    it 'returns no content for JSON requests' do
      person = create_managed_person_for(users(:admin), 'Delete JSON')

      delete person_path(person), as: :json

      expect(response).to have_http_status(:no_content)
    end

    it 'preserves medication history and renders a Turbo validation error when deletion is restricted' do
      person = create_managed_person_for(users(:admin), 'Retained History')
      medication = create(:medication, household: person.household)
      schedule = create(:schedule, household: person.household, person: person, medication: medication)
      create(:medication_take, :for_schedule, household: person.household, schedule: schedule)

      delete person_path(person), headers: { 'Accept' => 'text/vnd.turbo-stream.html' }

      expect(response).to have_http_status(:unprocessable_content)
      expect(response.media_type).to eq('text/vnd.turbo-stream.html')
      expect(Person.exists?(person.id)).to be(true)
      expect(Schedule.exists?(schedule.id)).to be(true)
      expect(response.body).to include('target="flash"')
    end
  end

  describe 'GET /people/:id/add_medication' do
    before { sign_in(users(:admin)) }

    it 'renders medication workflow options with the selected medication id preserved' do
      person = people(:john)
      medication = medications(:paracetamol)

      get add_medication_person_path(person), params: { source: 'finder', medication_id: medication.id }

      expect(response).to have_http_status(:ok)
      expect(response.body).to include(new_person_schedule_path(person, medication_id: medication.id))
      expect(response.body).to include(new_person_person_medication_path(person, medication_id: medication.id))
    end
  end

  def household_target(target)
    household = Household.find_by!(slug: default_url_options.fetch(:household_slug))

    "household_#{household.id}_#{target}"
  end

  def current_membership
    household = Household.find_by!(slug: default_url_options.fetch(:household_slug))

    household.household_memberships.find_by!(account: Account.find_by!(email: users(:jane).email_address))
  end

  def create_managed_person_for(user, name)
    household = Household.find_by!(slug: default_url_options.fetch(:household_slug))
    membership = household.household_memberships.find_by!(account: user.person.account)
    person = create(:person, household: household, name: name)
    household.person_access_grants.create!(
      household_membership: membership,
      person: person,
      access_level: :manage,
      relationship_type: :family_member,
      granted_by_membership: membership
    )
    person
  end
end
