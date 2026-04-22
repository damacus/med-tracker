# frozen_string_literal: true

require 'nokogiri'

module NhsDmd
  class ReleaseImport
    PROGRESS_BATCH_SIZE = 250

    Result = Struct.new(:imported_count, :skipped_count, keyword_init: true)

    def import(release_dir, progress_callback: nil)
      dir = Pathname.new(release_dir)
      ampp_file = glob_one(dir, 'f_ampp2_3*.xml')
      gtin_file = find_gtin_file(dir)

      counts = build_counts(ampp_file, gtin_file)
      emit_initial_progress(progress_callback, counts)

      names = parse_ampp_names(ampp_file, counts:, progress_callback: progress_callback)
      emit_gtin_start_progress(progress_callback, counts)

      import_gtins(gtin_file, names, counts:, progress_callback: progress_callback)
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

    def parse_ampp_names(path, counts:, progress_callback:)
      each_ampp_doc(path).with_object({}) do |doc, names|
        appid = node_text(doc, 'APPID')
        name = node_text(doc, 'NM')
        track_ampp_progress(counts, progress_callback)
        next if appid.blank? || name.blank?

        names[appid] = name
      end
    ensure
      emit_progress(counts, progress_callback, force: true, message: ampp_progress_message(counts))
    end

    def import_gtins(path, names, counts:, progress_callback:)
      today = Time.zone.today

      each_ampp_doc(path) do |doc|
        import_ampp_gtins(doc, names:, today:, counts:, progress_callback:)
      end

      emit_progress(counts, progress_callback, force: true, message: gtin_progress_message(counts))

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

    def import_ampp_gtins(doc, names:, today:, counts:, progress_callback:)
      amppid = node_text(doc, 'AMPPID')
      if amppid.blank?
        process_unmatched_gtins(doc, counts, progress_callback)
        return
      end

      display = names[amppid]
      doc.css('GTINDATA').each do |gtin_data|
        import_gtin_data(gtin_data, amppid:, display:, today:, counts:)
        emit_progress(counts, progress_callback, message: gtin_progress_message(counts))
      end
    end

    def import_gtin_data(gtin_data, amppid:, display:, today:, counts:)
      mark_gtin_processed(counts)
      gtin = node_text(gtin_data, 'GTIN')
      return skip_record(counts) if gtin.blank? || expired?(gtin_data, today)
      return skip_record(counts) if display.blank?

      persist(
        gtin: NhsDmdBarcode.normalize_gtin(gtin),
        code: amppid,
        display: display,
        system: 'https://dmd.nhs.uk',
        concept_class: 'AMPP'
      )
      counts[:imported] += 1
    end

    def skip_record(counts)
      counts[:skipped] += 1
      nil
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

    def count_gtin_records(path) = count_records(path, 'GTINDATA')

    def count_ampp_records(path) = count_records(path, 'AMPP')

    def count_records(path, node_name)
      Nokogiri::XML::Reader(File.open(path)).count do |node|
        node.name == node_name && node.node_type == Nokogiri::XML::Reader::TYPE_ELEMENT
      end
    end

    def emit_progress(counts, progress_callback, message:, force: false)
      return unless progress_callback
      return unless force || progress_batch_reached?(counts)

      progress_callback.call(
        status: :importing,
        message: message,
        total_records: counts[:total],
        processed_records: counts[:processed],
        imported_count: counts[:imported],
        skipped_count: counts[:skipped]
      )
    end

    def emit_initial_progress(progress_callback, counts)
      return unless progress_callback

      progress_callback.call(counting_progress(counts))
      progress_callback.call(starting_ampp_progress(counts))
    end

    def process_unmatched_gtins(doc, counts, progress_callback)
      doc.css('GTINDATA').each do
        mark_gtin_processed(counts)
        counts[:skipped] += 1
        emit_progress(counts, progress_callback, message: gtin_progress_message(counts))
      end
    end

    def progress_batch_reached?(counts)
      counts[:processed].positive? && (counts[:processed] % PROGRESS_BATCH_SIZE).zero?
    end

    def counting_progress(counts)
      initial_progress_payload(
        status: :counting,
        message: "Counted #{counts[:ampp_total]} AMPP records and #{counts[:gtin_total]} GTIN records",
        total_records: counts[:total]
      )
    end

    def starting_ampp_progress(counts)
      initial_progress_payload(
        status: :importing,
        message: 'Starting AMPP name import',
        total_records: counts[:total]
      )
    end

    def emit_gtin_start_progress(progress_callback, counts)
      return unless progress_callback

      progress_callback.call(
        initial_progress_payload(
          status: :importing,
          message: 'Starting GTIN import',
          total_records: counts[:total]
        ).merge(processed_records: counts[:processed])
      )
    end

    def initial_progress_payload(status:, message:, total_records:)
      {
        status: status,
        message: message,
        total_records: total_records,
        processed_records: 0,
        imported_count: 0,
        skipped_count: 0
      }
    end

    def build_counts(ampp_file, gtin_file)
      ampp_total = count_ampp_records(ampp_file)
      gtin_total = count_gtin_records(gtin_file)

      {
        ampp_total: ampp_total,
        gtin_total: gtin_total,
        total: ampp_total + gtin_total,
        processed: 0,
        ampp_processed: 0,
        gtin_processed: 0,
        imported: 0,
        skipped: 0
      }
    end

    def track_ampp_progress(counts, progress_callback)
      counts[:ampp_processed] += 1
      counts[:processed] += 1
      emit_progress(counts, progress_callback, message: ampp_progress_message(counts))
    end

    def mark_gtin_processed(counts)
      counts[:gtin_processed] += 1
      counts[:processed] += 1
    end

    def ampp_progress_message(counts)
      "Processed #{counts[:ampp_processed]} AMPP records"
    end

    def gtin_progress_message(counts)
      "Processed #{counts[:processed]} import records"
    end
  end
end
