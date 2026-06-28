// schema.rs — the Tantivy schema: one document per file.
//
//   path  STRING|STORED  the file path (also the delete key for re-indexing)
//   enc   STRING|STORED  encoding name, so the file can be re-decoded at search time
//   tri   indexed-only   distinct char-trigrams of the (lowercased, decoded) content
//   mtime U64|STORED|FAST modification time in ms, for incremental sync

use tantivy::schema::{
    Field, IndexRecordOption, Schema, TextFieldIndexing, TextOptions, FAST, STORED, STRING,
};

#[derive(Clone, Copy)]
pub struct Fields {
    pub path: Field,
    pub enc: Field,
    pub tri: Field,
    pub mtime: Field,
}

pub fn build_schema() -> (Schema, Fields) {
    let mut sb = Schema::builder();
    let path = sb.add_text_field("path", STRING | STORED);
    let enc = sb.add_text_field("enc", STRING | STORED);
    let tri_indexing = TextFieldIndexing::default()
        .set_tokenizer("tri")
        .set_index_option(IndexRecordOption::Basic);
    let tri = sb.add_text_field("tri", TextOptions::default().set_indexing_options(tri_indexing));
    let mtime = sb.add_u64_field("mtime", STORED | FAST);
    (sb.build(), Fields { path, enc, tri, mtime })
}
