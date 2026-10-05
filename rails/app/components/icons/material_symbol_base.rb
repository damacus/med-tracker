# frozen_string_literal: true

module Components
  module Icons
    class MaterialSymbolBase < Base
      private

      def default_attrs
        {
          xmlns: 'http://www.w3.org/2000/svg',
          width: size.to_s,
          height: size.to_s,
          viewBox: '0 -960 960 960',
          fill: 'currentColor',
          class: "material-symbol material-symbol-#{self.class.name.demodulize.underscore.dasherize}"
        }
      end
    end
  end
end
