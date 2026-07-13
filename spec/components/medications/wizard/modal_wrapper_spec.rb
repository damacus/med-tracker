# frozen_string_literal: true

require 'rails_helper'

RSpec.describe Components::Medications::Wizard::ModalWrapper, type: :component do
  fixtures :locations, :medications, :people

  [
    described_class,
    Components::Medications::Wizard::SlideOverWrapper
  ].each do |wrapper_class|
    it "translates and de-duplicates close controls in #{wrapper_class.name.demodulize}" do
      rendered = I18n.with_locale(:cy) do
        render_inline(
          wrapper_class.new(
            medication: medications(:paracetamol),
            locations: [locations(:home)],
            people: [people(:john)]
          )
        )
      end
      close_links = rendered.css('a[aria-label="Cau"]')

      expect(close_links.size).to eq(2)
      expect(close_links.css('svg[aria-hidden="true"]')).to be_present
      expect(close_links.css('.sr-only')).to be_empty
    end
  end
end
