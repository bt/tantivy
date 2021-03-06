use error::TantivyError;
use fst_regex::Regex;
use query::{AutomatonWeight, Query, Weight};
use schema::Field;
use std::clone::Clone;
use Result;
use Searcher;

// A Regex Query matches all of the documents
/// containing a specific term that matches
/// a regex pattern
/// A Fuzzy Query matches all of the documents
/// containing a specific term that is within
/// Levenshtein distance
///
/// ```rust
/// #[macro_use]
/// extern crate tantivy;
/// use tantivy::schema::{SchemaBuilder, TEXT};
/// use tantivy::{Index, Result, Term};
/// use tantivy::collector::{CountCollector, TopCollector, chain};
/// use tantivy::query::RegexQuery;
///
/// # fn main() { example().unwrap(); }
/// fn example() -> Result<()> {
///     let mut schema_builder = SchemaBuilder::new();
///     let title = schema_builder.add_text_field("title", TEXT);
///     let schema = schema_builder.build();
///     let index = Index::create_in_ram(schema);
///     {
///         let mut index_writer = index.writer(3_000_000)?;
///         index_writer.add_document(doc!(
///             title => "The Name of the Wind",
///         ));
///         index_writer.add_document(doc!(
///             title => "The Diary of Muadib",
///         ));
///         index_writer.add_document(doc!(
///             title => "A Dairy Cow",
///         ));
///         index_writer.add_document(doc!(
///             title => "The Diary of a Young Girl",
///         ));
///         index_writer.commit().unwrap();
///     }
///
///     index.load_searchers()?;
///     let searcher = index.searcher();
///
///     {
///         let mut top_collector = TopCollector::with_limit(2);
///         let mut count_collector = CountCollector::default();
///         {
///             let mut collectors = chain().push(&mut top_collector).push(&mut count_collector);
///             let term = Term::from_field_text(title, "Diary");
///             let query = RegexQuery::new("d[ai]{2}ry".to_string(), title);
///             searcher.search(&query, &mut collectors).unwrap();
///         }
///         assert_eq!(count_collector.count(), 3);
///         assert!(top_collector.at_capacity());
///     }
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct RegexQuery {
    regex_pattern: String,
    field: Field,
}

impl RegexQuery {
    /// Creates a new Fuzzy Query
    pub fn new(regex_pattern: String, field: Field) -> RegexQuery {
        RegexQuery {
            regex_pattern,
            field,
        }
    }

    fn specialized_weight(&self) -> Result<AutomatonWeight<Regex>> {
        let automaton = Regex::new(&self.regex_pattern)
            .map_err(|_| TantivyError::InvalidArgument(self.regex_pattern.clone()))?;

        Ok(AutomatonWeight::new(self.field, automaton))
    }
}

impl Query for RegexQuery {
    fn weight(&self, _searcher: &Searcher, _scoring_enabled: bool) -> Result<Box<Weight>> {
        Ok(Box::new(self.specialized_weight()?))
    }
}

#[cfg(test)]
mod test {
    use super::RegexQuery;
    use collector::TopCollector;
    use schema::SchemaBuilder;
    use schema::TEXT;
    use tests::assert_nearly_equals;
    use Index;

    #[test]
    pub fn test_regex_query() {
        let mut schema_builder = SchemaBuilder::new();
        let country_field = schema_builder.add_text_field("country", TEXT);
        let schema = schema_builder.build();
        let index = Index::create_in_ram(schema);
        {
            let mut index_writer = index.writer_with_num_threads(1, 10_000_000).unwrap();
            index_writer.add_document(doc!(
                country_field => "japan",
            ));
            index_writer.add_document(doc!(
                country_field => "korea",
            ));
            index_writer.commit().unwrap();
        }
        index.load_searchers().unwrap();
        let searcher = index.searcher();
        {
            let mut collector = TopCollector::with_limit(2);
            let regex_query = RegexQuery::new("jap[ao]n".to_string(), country_field);
            searcher.search(&regex_query, &mut collector).unwrap();
            let scored_docs = collector.top_docs();
            assert_eq!(scored_docs.len(), 1, "Expected only 1 document");
            let (score, _) = scored_docs[0];
            assert_nearly_equals(1f32, score);
        }
        {
            let mut collector = TopCollector::with_limit(2);
            let regex_query = RegexQuery::new("jap[A-Z]n".to_string(), country_field);
            searcher.search(&regex_query, &mut collector).unwrap();
            let scored_docs = collector.top_docs();
            assert_eq!(scored_docs.len(), 0, "Expected ZERO document");
        }
    }
}
