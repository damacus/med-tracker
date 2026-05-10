# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Layouts::FloatingActionMenu, type: :component do
  fixtures :accounts, :people, :users

  let(:carer_user) { users(:carer) }
  let(:admin_user) { users(:admin) }

  def render_menu(user:, path:)
    vc = view_context
    resolver = method(:policy_for)
    vc.singleton_class.define_method(:current_user) { user }
    vc.singleton_class.define_method(:policy) { |record| resolver.call(user, record) }
    allow(vc.request).to receive(:path).and_return(path)

    html = vc.render(described_class.new(current_user: user))
    Nokogiri::HTML::DocumentFragment.parse(html)
  end

  def policy_for(user, record)
    case record
    when Person then PersonPolicy.new(user, record)
    when Location then LocationPolicy.new(user, record)
    when Class
      return PersonPolicy.new(user, Person.new) if record == Person

      raise "Unexpected policy lookup for #{record}"
    else
      raise "Unexpected policy lookup for #{record.class}"
    end
  end

  it 'renders quick actions on allowed pages' do
    rendered = render_menu(user: carer_user, path: Rails.application.routes.url_helpers.medications_path)

    expect(rendered.css('button[aria-label="Open quick actions"]')).to be_present
    expect(rendered.text).to include('Add Medication', 'Add Person')
  end

  it 'renders on each allowed index page' do
    allowed_paths = [
      Rails.application.routes.url_helpers.people_path,
      Rails.application.routes.url_helpers.medications_path,
      Rails.application.routes.url_helpers.locations_path,
      Rails.application.routes.url_helpers.schedules_path
    ]

    allowed_paths.each do |path|
      rendered = render_menu(user: carer_user, path:)

      expect(rendered.css('button[aria-label="Open quick actions"]')).to be_present
    end
  end

  it 'hides the menu on disallowed pages' do
    rendered = render_menu(user: admin_user, path: Rails.application.routes.url_helpers.profile_path)

    expect(rendered.text.strip).to be_empty
  end

  it 'hides the menu on forms, workflow, profile, and admin pages' do
    hidden_paths = [
      Rails.application.routes.url_helpers.root_path,
      Rails.application.routes.url_helpers.dashboard_path,
      Rails.application.routes.url_helpers.profile_path,
      Rails.application.routes.url_helpers.admin_root_path,
      Rails.application.routes.url_helpers.new_person_path,
      Rails.application.routes.url_helpers.new_location_path,
      Rails.application.routes.url_helpers.add_medication_path
    ]

    hidden_paths.each do |path|
      rendered = render_menu(user: admin_user, path:)

      expect(rendered.text.strip).to be_empty
    end
  end

  it 'renders policy-permitted quick actions only' do
    carer_rendered = render_menu(user: carer_user, path: Rails.application.routes.url_helpers.medications_path)
    admin_rendered = render_menu(user: admin_user, path: Rails.application.routes.url_helpers.medications_path)

    expect(carer_rendered.text).to include('Add Medication', 'Add Person')
    expect(carer_rendered.text).not_to include('Add Location')
    expect(admin_rendered.text).to include('Add Medication', 'Add Location')
    expect(admin_rendered.text).not_to include('Add Person')
  end
end
