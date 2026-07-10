# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Admin::AuditLogs::ShowView, type: :component do
  fixtures :accounts, :people, :users, :locations, :location_memberships, :medications, :dosages, :schedules

  let(:admin) { users(:admin) }
  let(:person) { people(:john) }
  let(:version) do
    PaperTrail.request.whodunnit = admin.id
    person.update!(name: 'Updated Name')
    person.versions.last
  end

  after do
    PaperTrail.request.whodunnit = nil
  end

  describe 'initialization' do
    it 'accepts a version' do
      view = view_for(version)

      expect(view.version).to eq(version)
      expect(view.detail).to be_a(Admin::AuditLogDetailQuery::Result)
    end
  end

  describe 'helper methods' do
    subject(:view) { view_for(version) }

    describe '#user_name' do
      it 'returns the user name when whodunnit is present' do
        expect(view.send(:user_name)).to eq(admin.name)
      end

      it 'returns System when whodunnit is blank' do
        allow(version).to receive(:whodunnit).and_return(nil)
        expect(view.send(:user_name)).to eq(I18n.t('admin.audit_logs.show.system'))
      end

      it 'returns User #ID when user is not found' do
        allow(version).to receive(:whodunnit).and_return('999999')
        expect(view.send(:user_name)).to eq('User #999999')
      end
    end

    describe '#format_object' do
      it 'formats YAML object as JSON' do
        yaml_string = "---\nname: Test\nid: 123\n"
        result = view.send(:format_object, yaml_string)

        expect(result).to include('"name"')
        expect(result).to include('"Test"')
      end

      it 'returns original string on parse error' do
        invalid_yaml = 'not valid yaml: ['
        result = view.send(:format_object, invalid_yaml)

        expect(result).to eq(invalid_yaml)
      end
    end

    describe '#filter_sensitive_fields' do
      it 'removes password_digest from data' do
        data = { 'name' => 'Test', 'password_digest' => 'secret123' }
        result = view.send(:filter_sensitive_fields, data)

        expect(result).not_to have_key('password_digest')
        expect(result).to have_key('name')
      end

      it 'removes password_hash from data' do
        data = { 'name' => 'Test', 'password_hash' => 'secret123' }
        result = view.send(:filter_sensitive_fields, data)

        expect(result).not_to have_key('password_hash')
        expect(result).to have_key('name')
      end

      it 'removes token and token_digest from data' do
        data = { 'name' => 'Test', 'token' => 'raw', 'token_digest' => 'digest' }
        result = view.send(:filter_sensitive_fields, data)

        expect(result).not_to have_key('token')
        expect(result).not_to have_key('token_digest')
      end

      it 'returns non-hash data unchanged' do
        expect(view.send(:filter_sensitive_fields, 'string')).to eq('string')
        expect(view.send(:filter_sensitive_fields, nil)).to be_nil
      end
    end

    describe '#description_for_new_state' do
      it 'returns create description for create events' do
        allow(version).to receive(:event).and_return('create')
        expect(view.send(:description_for_new_state)).to eq(I18n.t('admin.audit_logs.show.new_state_create'))
      end

      it 'returns update description for update events' do
        allow(version).to receive(:event).and_return('update')
        expect(view.send(:description_for_new_state)).to eq(I18n.t('admin.audit_logs.show.new_state_update'))
      end
    end

    describe '#compute_new_state' do
      it 'returns the next version object when there is a next version' do
        person.update!(name: 'Another Update')
        result = view.send(:compute_new_state)
        expect(result).to be_present
      end

      it 'returns the current record attributes when there is no next version' do
        result = view.send(:compute_new_state)
        expect(result).to be_a(Hash)
        expect(result['name']).to eq(person.name)
      end
    end

    describe '#format_new_state' do
      it 'formats YAML string as JSON' do
        yaml_string = "---\nname: Test\n"
        result = view.send(:format_new_state, yaml_string)
        expect(result).to include('"name"')
      end

      it 'formats Hash as JSON' do
        hash = { 'name' => 'Test' }
        result = view.send(:format_new_state, hash)
        expect(result).to include('"name"')
      end

      it 'converts other types to string' do
        expect(view.send(:format_new_state, 123)).to eq('123')
      end
    end

    describe '#event_summary_items' do
      let(:version) do
        PaperTrail::Version.new(
          item_type: 'ExternalMedicineLookup',
          item_id: 0,
          event: 'nhs_website_content/medicine_guidance_lookup',
          object: {
            query: 'Panadol 500mg tablets',
            result_status: 'success',
            result_count: 1,
            matched_title: 'Paracetamol for adults',
            matched_url: 'https://www.nhs.uk/medicines/paracetamol-for-adults/'
          }.to_json
        )
      end

      it 'promotes external lookup context into readable summary items' do
        expect(view.send(:event_summary_items)).to include(
          ['Lookup', 'Panadol 500mg tablets'],
          %w[Result Success],
          %w[Matches 1],
          ['Matched guidance', 'Paracetamol for adults'],
          ['Matched URL', 'https://www.nhs.uk/medicines/paracetamol-for-adults/']
        )
      end
    end
  end

  describe 'medication take summary' do
    let(:medication_take_schedule) { schedules(:john_paracetamol) }
    let(:medication_take_version) do
      medication_take_schedule.medication.update!(current_supply: 100)
      PaperTrail.request.whodunnit = admin.id
      take = MedicationTake.create!(
        schedule: medication_take_schedule,
        taken_at: Time.zone.parse('2026-06-07 12:10:00'),
        dose_amount: medication_take_schedule.dose_amount,
        dose_unit: medication_take_schedule.dose_unit
      )
      PaperTrail::Version.where(item_type: 'MedicationTake', item_id: take.id).last
    end

    it 'promotes medication and patient details', :aggregate_failures do
      expect(medication_take_summary).to be_present
      expect(medication_take_summary.text).to include('Medication')
      expect(medication_take_summary.text).to include(medication_take_schedule.medication.display_name)
      expect(medication_take_summary.text).to include('Patient')
      expect(medication_take_summary.text).to include(medication_take_schedule.person.name)
    end

    it 'promotes administered time and logger details', :aggregate_failures do
      expect(medication_take_summary.text).to include('Administered at')
      expect(medication_take_summary.text).to include('12:10')
      expect(medication_take_summary.text).to include('Logged by')
      expect(medication_take_summary.text).to include(admin.name)
    end

    it 'enriches the state payload with readable medication context', :aggregate_failures do
      payload = medication_take_payload

      expect(payload['medication_name']).to eq(medication_take_schedule.medication.display_name)
      expect(payload['patient_name']).to eq(medication_take_schedule.person.name)
      expect(payload['source']).to eq(
        'id' => medication_take_schedule.id,
        'type' => 'Schedule',
        'medication' => medication_take_schedule.medication.display_name,
        'person' => medication_take_schedule.person.name
      )
      expect(payload['logged_by_name']).to eq(admin.name)
    end

    it 'does not look up the logged-by user while rendering' do
      version = medication_take_version
      view = view_for(version)

      expect(count_user_queries { render_inline(view) }).to eq(0)
    end

    it 'does not execute SQL while rendering an audit detail' do
      version = medication_take_version
      view = view_for(version)

      expect(count_queries { render_inline(view) }).to eq(0)
    end

    def medication_take_summary
      rendered = render_inline(view_for(medication_take_version))

      rendered.at_css('[data-testid="audit-log-medication-take-summary"]')
    end

    def medication_take_payload
      rendered = render_inline(view_for(medication_take_version))
      JSON.parse(rendered.css('pre code').last.text)
    end

    def count_user_queries(&)
      count = 0

      subscriber = lambda do |_name, _start, _finish, _id, payload|
        sql = payload[:sql]
        next if payload[:cached] || payload[:name] == 'SCHEMA'

        count += 1 if sql.include?('"users"') && sql.include?('"users"."id"')
      end

      ActiveSupport::Notifications.subscribed(subscriber, 'sql.active_record', &)
      count
    end

    def count_queries(&)
      count = 0
      subscriber = lambda do |_name, _start, _finish, _id, payload|
        next if payload[:cached] || payload[:name] == 'SCHEMA'

        count += 1
      end

      ActiveSupport::Notifications.subscribed(subscriber, 'sql.active_record', &)
      count
    end
  end

  def view_for(version)
    detail = Admin::AuditLogDetailQuery.new(version: version).call
    described_class.new(version: version, detail: detail)
  end
end
