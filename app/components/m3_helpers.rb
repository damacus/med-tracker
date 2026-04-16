# frozen_string_literal: true

module Components
  module M3Helpers
    def m3_button(...) = render(Components::M3::Button.new(...))
    def m3_badge(...) = render(Components::M3::Badge.new(...))
    def m3_card(...) = render(Components::M3::Card.new(...))
    def m3_input(...) = render(Components::M3::Input.new(...))
    def m3_link(...) = render(Components::M3::Link.new(...))
    def m3_heading(...) = render(Components::M3::Heading.new(...))
    def m3_text(...) = render(Components::M3::Text.new(...))

    def m3_card_header(...) = render(Components::M3::CardHeader.new(...))
    def m3_card_title(...) = render(Components::M3::CardTitle.new(...))
    def m3_card_description(...) = render(Components::M3::CardDescription.new(...))
    def m3_card_content(...) = render(Components::M3::CardContent.new(...))
    def m3_card_footer(...) = render(Components::M3::CardFooter.new(...))
  end
end
