# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Admin::AuditLogs::ShowView, type: :component do
  fixtures :accounts, :people, :users

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
      view = described_class.new(version: version)

      expect(view.version).to eq(version)
    end
  end

  describe 'helper methods' do
    subject(:view) { described_class.new(version: version) }

    describe '#user_name' do
      it 'returns the user name when whodunnit is present' do
        expect(view.send(:user_name)).to eq(admin.name)
      end

      it 'returns System when whodunnit is blank' do
        allow(version).to receive(:whodunnit).and_return(nil)
        expect(view.send(:user_name)).to eq('System')
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

      it 'returns non-hash data unchanged' do
        expect(view.send(:filter_sensitive_fields, 'string')).to eq('string')
        expect(view.send(:filter_sensitive_fields, nil)).to be_nil
      end
    end

    describe '#description_for_new_state' do
      it 'returns create description for create events' do
        allow(version).to receive(:event).and_return('create')
        expect(view.send(:description_for_new_state)).to eq('The state of the record when it was created')
      end

      it 'returns update description for update events' do
        allow(version).to receive(:event).and_return('update')
        expect(view.send(:description_for_new_state)).to eq('The state of the record after this change')
      end
    end

    describe '#compute_new_state' do
      context 'when there is a next version' do
        let!(:next_version) do
          person.update!(name: 'Another Update')
          person.versions.last
        end

        it 'returns the next version object' do
          result = view.send(:compute_new_state)
          expect(result).to be_present
        end
      end

      context 'when there is no next version' do
        it 'returns the current record attributes' do
          result = view.send(:compute_new_state)
          expect(result).to be_a(Hash)
          expect(result['name']).to eq(person.name)
        end
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
  end
end
