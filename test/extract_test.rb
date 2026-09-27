# frozen_string_literal: true

require "minitest/autorun"
require "pathname"
require "stringio"
require "pdf_oxide_structured"

class ExtractTest < Minitest::Test
  # Extracts spans from a one-page native-text PDF given as a String or a Pathname.
  def test_extracts_a_string_path_and_a_pathname
    path = File.expand_path("fixtures/hello.pdf", __dir__)

    all_pages = PdfOxideStructured.extract(path)
    assert_equal 1, all_pages["page_count"]
    assert_equal [0], all_pages["pages"].map { |page| page["page_number"] }
    assert_includes span_text(all_pages), "Hello World"
    assert_equal path, all_pages["file"]

    from_pathname = PdfOxideStructured.extract(Pathname.new(path))
    assert_includes span_text(from_pathname), "Hello World"
    assert_equal path, from_pathname["file"]
  end

  # Reads a File and a StringIO without going through the official pdf_oxide gem.
  def test_extracts_a_file_and_a_stringio
    path = File.expand_path("fixtures/hello.pdf", __dir__)

    File.open(path, "rb") do |file|
      extracted = PdfOxideStructured.extract(file)
      assert_includes span_text(extracted), "Hello World"
      assert_equal path, extracted["file"]
    end

    extracted = PdfOxideStructured.extract(StringIO.new(File.binread(path)))
    assert_includes span_text(extracted), "Hello World"
    assert_equal "<in-memory>", extracted["file"]
  end

  # Page 0 matches every page, and a page past the end raises.
  def test_page_zero_matches_all_pages_and_out_of_range_raises
    path = File.expand_path("fixtures/hello.pdf", __dir__)

    all_pages = PdfOxideStructured.extract(path)
    one_page = PdfOxideStructured.extract(path, page: 0)
    assert_equal [0], one_page["pages"].map { |page| page["page_number"] }
    assert_equal all_pages["pages"], one_page["pages"]

    error = assert_raises(RuntimeError) { PdfOxideStructured.extract(path, page: 3) }
    assert_match(/out of bounds/, error.message)
  end

  # Splits native text into separate words from a String or a Pathname.
  def test_extract_words_from_a_string_path_and_a_pathname
    path = File.expand_path("fixtures/hello.pdf", __dir__)

    all_pages = PdfOxideStructured.extract_words(path)
    assert_equal 1, all_pages["page_count"]
    assert_equal [0], all_pages["pages"].map { |page| page["page_number"] }
    assert_includes word_texts(all_pages), "Hello"
    assert_includes word_texts(all_pages), "World"
    assert_nil all_pages["pages"].first["spans"]
    assert_equal path, all_pages["file"]

    from_pathname = PdfOxideStructured.extract_words(Pathname.new(path))
    assert_includes word_texts(from_pathname), "Hello"
    assert_includes word_texts(from_pathname), "World"
    assert_equal path, from_pathname["file"]
  end

  # Reads a File and a StringIO for words without going through the official pdf_oxide gem.
  def test_extract_words_from_a_file_and_a_stringio
    path = File.expand_path("fixtures/hello.pdf", __dir__)

    File.open(path, "rb") do |file|
      extracted = PdfOxideStructured.extract_words(file)
      assert_includes word_texts(extracted), "Hello"
      assert_includes word_texts(extracted), "World"
      assert_equal path, extracted["file"]
    end

    extracted = PdfOxideStructured.extract_words(StringIO.new(File.binread(path)))
    assert_includes word_texts(extracted), "Hello"
    assert_includes word_texts(extracted), "World"
    assert_equal "<in-memory>", extracted["file"]
  end

  # Word page 0 matches every page, and a page past the end raises.
  def test_extract_words_page_zero_matches_all_pages_and_out_of_range_raises
    path = File.expand_path("fixtures/hello.pdf", __dir__)

    all_pages = PdfOxideStructured.extract_words(path)
    one_page = PdfOxideStructured.extract_words(path, page: 0)
    assert_equal [0], one_page["pages"].map { |page| page["page_number"] }
    assert_equal all_pages["pages"], one_page["pages"]

    error = assert_raises(RuntimeError) { PdfOxideStructured.extract_words(path, page: 3) }
    assert_match(/out of bounds/, error.message)
  end

  # Joins span text from an extract result.
  def span_text(extracted)
    extracted["pages"].flat_map { |page| page["spans"].map { |span| span["text"] } }.join(" ")
  end

  # Returns each word's text from an extract_words result.
  def word_texts(extracted)
    extracted["pages"].flat_map { |page| page["words"].map { |word| word["text"] } }
  end
end
