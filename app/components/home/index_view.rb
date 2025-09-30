# frozen_string_literal: true

module Components
  module Home
    class IndexView < Components::Base
      def view_template
        div(class: 'container') do
          div(class: 'page-header') do
            h1(class: 'page-title') { 'Medicine Tracker' }
            p(class: 'page-subtitle') { 'Welcome to your personal medicine tracking application.' }
          end

          div(class: 'item-grid') do
            link_to(medicines_path, class: 'item-card') do
              h2(class: 'item-card__title') { 'Medicines' }
              p(class: 'item-card__detail') { 'Manage your medicine inventory' }
            end

            link_to(people_path, class: 'item-card') do
              h2(class: 'item-card__title') { 'People' }
              p(class: 'item-card__detail') { 'Manage people and their prescriptions' }
            end
          end
        end
      end
    end
  end
end
