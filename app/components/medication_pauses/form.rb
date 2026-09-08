module Components
  module MedicationPauses
    class Form < Components::Base
      include Phlex::Rails::Helpers::TurboFrameTag

      def initialize(source:, attributes: {}, error: nil, modal: false)
        @source = source
        @attributes = attributes
        @error = error
        @modal = modal
        super()
      end

      def view_template
        if modal
          turbo_frame_tag 'modal' do
            Dialog(open: true) do
              DialogContent(size: :md) do
                DialogHeader do
                  DialogTitle { t('medication_pauses.title') }
                  DialogDescription { source.medication.display_name }
                end
                DialogMiddle { render_form }
              end
            end
          end
        else
          div(class: 'mx-auto max-w-xl space-y-6 p-6') do
            Heading(level: 1) { t('medication_pauses.title') }
            Text { source.medication.display_name }
            render_form
          end
        end
      end

      private

      attr_reader :source, :attributes, :error, :modal

      def render_form
        form_with(url: pause_path, method: :patch, class: 'space-y-6', data: { turbo_frame: '_top' }) do
          Alert(variant: :destructive, role: :alert, id: 'pause-error') { plain error } if error
          FormField do
            FormFieldLabel(for: 'pause-reason') { t('medication_pauses.reason') }
            m3_select(id: 'pause-reason', name: 'pause_period[reason]', required: true,
                      aria: { invalid: error.present?, describedby: error ? 'pause-error' : nil }) do
              option(value: '') { t('medication_pauses.choose_reason') }
              MedicationPausePeriod::PUBLIC_REASONS.each do |reason|
                option(value: reason, selected: attributes[:reason] == reason) do
                  t("medication_pauses.reasons.#{reason}")
                end
              end
            end
          end
          FormField do
            FormFieldLabel(for: 'pause-note') { t('medication_pauses.note') }
            Textarea(id: 'pause-note', name: 'pause_period[note]', rows: 3) { attributes[:note].to_s }
          end
          Text(size: '2') { t('medication_pauses.timing') }
          div(class: 'flex justify-end gap-3') do
            if modal
              Button(type: :button, variant: :outline, data: { action: 'click->ruby-ui--dialog#dismiss' }) do
                t('medication_pauses.cancel')
              end
            else
              Link(href: person_path(source.person)) { t('medication_pauses.cancel') }
            end
            Button(type: :submit) { t('medication_pauses.submit') }
          end
        end
      end

      def pause_path
        if source.is_a?(Schedule)
          pause_person_schedule_path(source.person, source)
        else
          pause_person_person_medication_path(source.person, source)
        end
      end
    end
  end
end
