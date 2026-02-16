# frozen_string_literal: true

module Components
  module Layouts
    # Desktop navigation links
    class DesktopNav < Components::Base
      include Phlex::Rails::Helpers::LinkTo

      def view_template
        div(class: 'hidden md:flex items-center gap-6') do
          link_to(t('layouts.desktop_nav.medicines'), medicines_path,
                  class: 'nav__link text-sm font-medium transition-colors hover:text-primary')
          link_to(t('layouts.desktop_nav.people'), people_path,
                  class: 'nav__link text-sm font-medium transition-colors hover:text-primary')
          link_to(t('layouts.desktop_nav.medicine_finder'), medicine_finder_path,
                  class: 'nav__link text-sm font-medium transition-colors hover:text-primary')
        end
      end
    end
  end
end
