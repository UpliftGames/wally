use std::io::{BufRead, BufReader};
use std::path::PathBuf;

use fs_err::File;
use libwally::manifest::Manifest;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;

use tantivy::tokenizer::{LowerCaser, NgramTokenizer, TextAnalyzer};
use tantivy::{schema::*, IndexReader, ReloadPolicy};
use tantivy::{Index, IndexWriter};
use walkdir::{DirEntry, WalkDir};

static DOC_LIMIT: usize = 100;

pub struct SearchBackend {
    schema: Schema,
    package_index: PathBuf,
    writer: IndexWriter,
    reader: IndexReader,
    query_parser: QueryParser,
}

impl SearchBackend {
    pub fn new(package_index: PathBuf) -> anyhow::Result<Self> {
        let mut schema_builder = Schema::builder();

        let text_options = TextOptions::default()
            .set_indexing_options(
                TextFieldIndexing::default()
                    .set_tokenizer("ngram")
                    .set_index_option(IndexRecordOption::WithFreqsAndPositions),
            )
            .set_stored();

        schema_builder.add_text_field("scope", text_options.clone());
        schema_builder.add_text_field("name", text_options.clone());
        schema_builder.add_text_field("version", TEXT | STORED);
        schema_builder.add_text_field("description", text_options);

        let schema = schema_builder.build();
        let index = Index::create_in_ram(schema.clone());

        let analyzer = TextAnalyzer::from(NgramTokenizer::all_ngrams(2, 10)).filter(LowerCaser);
        index.tokenizers().register("ngram", analyzer);

        let writer = index.writer(50_000_000)?;

        let reader = index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommit)
            .try_into()?;

        let scope = schema.get_field("scope").unwrap();
        let name = schema.get_field("name").unwrap();
        let description = schema.get_field("description").unwrap();

        let query_parser = QueryParser::for_index(&index, vec![scope, name, description]);

        let mut backend = Self {
            schema,
            package_index,
            writer,
            reader,
            query_parser,
        };

        backend.crawl_packages()?;
        Ok(backend)
    }

    fn crawl_packages(&mut self) -> anyhow::Result<()> {
        let scope = self.schema.get_field("scope").unwrap();
        let name = self.schema.get_field("name").unwrap();
        let version = self.schema.get_field("version").unwrap();
        let description = self.schema.get_field("description").unwrap();

        println!("Crawling index...");

        for entry in WalkDir::new(&self.package_index)
            .min_depth(1)
            .into_iter()
            .filter_entry(|e| !is_config(e))
        {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                continue;
            }

            let file = File::open(path)?;
            let reader = BufReader::new(file);
            // TODO: last line only isn't good enough as it may be a pre-release etc
            let last_line = reader
                .lines()
                .last()
                .unwrap_or_else(|| panic!("Package file {} exists with no data!", path.display()));

            let manifest: Manifest = serde_json::from_str(&last_line?)?;
            let mut doc = Document::default();

            doc.add_text(scope, manifest.package.name.scope());
            doc.add_text(name, manifest.package.name.name());
            doc.add_text(version, manifest.package.version.to_string());

            if let Some(description_text) = manifest.package.description {
                doc.add_text(description, description_text);
            }

            self.writer.add_document(doc);
        }

        self.writer.commit()?;

        Ok(())
    }

    pub fn search(&self, query_input: &str) -> tantivy::Result<Vec<String>> {
        let searcher = self.reader.searcher();
        let query = self.query_parser.parse_query(query_input)?;
        let top_docs = searcher.search(&query, &TopDocs::with_limit(DOC_LIMIT))?;

        let mut docs = Vec::with_capacity(DOC_LIMIT);

        for (_score, doc_address) in top_docs {
            let retrieved_doc = searcher.doc(doc_address)?;
            docs.push(self.schema.to_json(&retrieved_doc));
        }

        Ok(docs)
    }
}

fn is_config(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.ends_with(".json") || s.ends_with(".toml") || s.starts_with('.'))
        .unwrap_or(false)
}
