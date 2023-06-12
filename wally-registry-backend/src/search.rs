use std::time::Instant;

use libwally::package_index::PackageIndex;
use libwally::package_name::PackageName;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;

use serde::{Deserialize, Serialize};
use tantivy::tokenizer::{LowerCaser, NgramTokenizer, TextAnalyzer};
use tantivy::{schema::*, IndexReader, ReloadPolicy};
use tantivy::{Index, IndexWriter};
use walkdir::{DirEntry, WalkDir};

static DOC_LIMIT: usize = 100;

pub struct SearchBackend {
    schema: Schema,
    writer: IndexWriter,
    reader: IndexReader,
    query_parser: QueryParser,
}

impl SearchBackend {
    pub fn new(package_index: &PackageIndex) -> anyhow::Result<Self> {
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
        schema_builder.add_text_field("versions", TEXT | STORED);
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

        let mut query_parser = QueryParser::for_index(&index, vec![scope, name, description]);
        query_parser.set_conjunction_by_default();
        query_parser.set_field_boost(scope, 3.0);
        query_parser.set_field_boost(name, 5.0);

        let mut backend = Self {
            schema,
            writer,
            reader,
            query_parser,
        };

        backend.crawl_packages(package_index)?;
        Ok(backend)
    }

    pub fn crawl_packages(&mut self, package_index: &PackageIndex) -> anyhow::Result<()> {
        let scope = self.schema.get_field("scope").unwrap();
        let name = self.schema.get_field("name").unwrap();
        let versions = self.schema.get_field("versions").unwrap();
        let description = self.schema.get_field("description").unwrap();

        println!("Crawling index...");
        let now = Instant::now();
        self.writer.delete_all_documents()?;

        for entry in WalkDir::new(package_index.path())
            .min_depth(1)
            .into_iter()
            .filter_entry(|e| !is_config(e))
        {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                continue;
            }

            let package_scope = path
                .parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap();
            let package_name = path.file_name().unwrap().to_str().unwrap();

            let metadata = package_index
                .get_package_metadata(&PackageName::new(package_scope, package_name)?)?;

            let mut doc = Document::default();

            for manifest in &(*metadata).versions {
                doc.add_text(versions, manifest.package.version.to_string());

                if !manifest.package.version.is_prerelease() {
                    doc.add_text(scope, manifest.package.name.scope());
                    doc.add_text(name, manifest.package.name.name());

                    if let Some(description_text) = &manifest.package.description {
                        doc.add_text(description, description_text);
                    }

                    break;
                }
            }

            self.writer.add_document(doc);
        }

        self.writer.commit()?;
        println!("Finished crawling in {}ms", now.elapsed().as_millis());

        Ok(())
    }

    pub fn search(&self, query_input: &str) -> tantivy::Result<Vec<DocResult>> {
        let searcher = self.reader.searcher();
        let query = self
            .query_parser
            .parse_query(&query_input.replace("/", " "))?;
        let top_docs = searcher.search(&query, &TopDocs::with_limit(DOC_LIMIT))?;

        let mut docs = Vec::with_capacity(DOC_LIMIT);

        for (_score, doc_address) in top_docs {
            let retrieved_doc = searcher.doc(doc_address)?;
            let retrieved_doc = self.schema.to_json(&retrieved_doc);
            let retrieved_doc: NativeDocResult = match serde_json::from_str(&retrieved_doc) {
                Ok(doc) => doc,
                Err(_) => continue,
            };
            docs.push(DocResult {
                scope: retrieved_doc.scope[0].clone(),
                name: retrieved_doc.name[0].clone(),
                versions: retrieved_doc.versions,
                description: retrieved_doc.description.map(|d| d[0].clone()),
            });
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

#[derive(Serialize, Deserialize)]
struct NativeDocResult {
    scope: Vec<String>,
    name: Vec<String>,
    versions: Vec<String>,
    description: Option<Vec<String>>,
}

#[derive(Serialize, Deserialize)]
pub struct DocResult {
    scope: String,
    name: String,
    versions: Vec<String>,
    description: Option<String>,
}
