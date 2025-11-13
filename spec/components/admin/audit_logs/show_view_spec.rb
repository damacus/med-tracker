# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Admin::AuditLogs::ShowView, type: :component do
  fixtures :users, :people

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
  end
end
