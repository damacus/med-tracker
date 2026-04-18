# frozen_string_literal: true

require 'nokogiri'

module NhsDmd
  class ReleaseImport
    Result = Struct.new(:imported_count, :skipped_count, keyword_init: true)

    def import(release_dir)
      dir = Pathname.new(release_dir)
      ampp_file = glob_one(dir, 'f_ampp2_3*.xml')
      gtin_file = find_gtin_file(dir)

      names = parse_ampp_names(ampp_file)
      import_gtins(gtin_file, names)
    end

    private

    def glob_one(dir, pattern)
      matches = Dir.glob(dir.join(pattern))
      raise ArgumentError, "No file matching #{pattern} in #{dir}" if matches.empty?

      matches.first
    end

    def find_gtin_file(dir)
      existing = Dir.glob(dir.join('f_gtin2_0*.xml'))
      return existing.first if existing.any?

      zip = Dir.glob(dir.join('*GTIN.zip')).first
      raise ArgumentError, "No GTIN XML or ZIP found in #{dir}" unless zip

      extract_dir = Dir.mktmpdir('dmd-gtin')
      extract_gtin_xml(zip, extract_dir)
      Dir.glob(File.join(extract_dir, 'f_gtin2_0*.xml')).first
    end

    def extract_gtin_xml(zip_path, dest)
      system('unzip', '-o', zip_path.to_s, 'f_gtin2_0*.xml', '-d', dest.to_s,
             exception: true)
    end

    def parse_ampp_names(path)
      each_ampp_doc(path).with_object({}) do |doc, names|
        appid = node_text(doc, 'APPID')
        name = node_text(doc, 'NM')
        next if appid.blank? || name.blank?

        names[appid] = name
      end
    end

    def import_gtins(path, names)
      counts = { imported: 0, skipped: 0 }
      today = Time.zone.today

      each_ampp_doc(path) do |doc|
        import_ampp_gtins(doc, names:, today:, counts:)
      end

      Result.new(imported_count: counts[:imported], skipped_count: counts[:skipped])
    end

    def each_ampp_doc(path)
      return enum_for(__method__, path) unless block_given?

      Nokogiri::XML::Reader(File.open(path)).each do |node|
        next unless ampp_element?(node)

        yield Nokogiri::XML(node.outer_xml)
      end
    end

    def ampp_element?(node)
      node.name == 'AMPP' && node.node_type == Nokogiri::XML::Reader::TYPE_ELEMENT
    end

    def import_ampp_gtins(doc, names:, today:, counts:)
      amppid = node_text(doc, 'AMPPID')
      return if amppid.blank?

      display = names[amppid]
      doc.css('GTINDATA').each do |gtin_data|
        import_gtin_data(gtin_data, amppid:, display:, today:, counts:)
      end
    end

    def import_gtin_data(gtin_data, amppid:, display:, today:, counts:)
      gtin = node_text(gtin_data, 'GTIN')
      return if gtin.blank? || expired?(gtin_data, today)

      if display.blank?
        counts[:skipped] += 1
        return
      end

      persist(
        gtin: NhsDmdBarcode.normalize_gtin(gtin),
        code: amppid,
        display: display,
        system: 'https://dmd.nhs.uk',
        concept_class: 'AMPP'
      )
      counts[:imported] += 1
    end

    def expired?(gtin_data, today)
      end_date = node_text(gtin_data, 'ENDDT')
      end_date.present? && Date.parse(end_date) <= today
    end

    def node_text(doc, selector)
      doc.at(selector)&.text
    end

    def persist(attrs)
      record = NhsDmdBarcode.find_or_initialize_by(gtin: attrs[:gtin])
      record.assign_attributes(attrs)
      record.save!
    end
  end
end
