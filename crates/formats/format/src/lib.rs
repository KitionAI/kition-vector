//! The native `.kvector` format.
//!
//! A `.kvector` file is UTF-8 JSON, gzip-compressed when saved with Use Compression (told apart
//! by its magic bytes on open):
//! ```json
//! { "format": "kvector", "version": 1, "generator": "Kition Vector 0.1.0",
//!   "preview": { "mime": "image/png", "data": "<base64>" },
//!   "profiles": { "<profile name>": { "mime": "application/vnd.iccprofile", "data": "<base64>" } },
//!   "pdf": { "mime": "application/pdf", "data": "<base64>" },
//!   "document": { …kition_vector_doc::Document… },
//!   "images": { "<key>": { "mime": "image/png", "data": "<base64>" } } }
//! ```
//! It is lossless for everything in the document model; foreign data lives in `document.unknown`
//! and document keys a newer version added are kept (`Document::extra`). Only the images the
//! document uses are written. `preview` (optional) is a PNG of the first artboard, at most
//! [`PREVIEW_MAX`] pixels on its longer side, for file browsers. `profiles` (optional, Embed ICC
//! Profiles) holds the ICC files of colour profiles the document is tagged with, for machines that
//! don't have them ([`load_file`] returns them). `pdf` (optional, Create PDF-Compatible File) is a
//! PDF of every artboard for apps that read PDF ([`pdf_content`]). An image only linked images
//! show is saved as its low-resolution preview (`"proxy": true`), unless saved with Include Linked
//! Files; the engine reads the linked file again when the document opens.
//!
//! Large data (images, placed documents, the PDF, profiles of at least [`INLINE_MAX`] bytes) goes
//! after the JSON instead of in it: the JSON is followed by [`BLOB_MAGIC`] and the bytes, and such
//! an entry says where they are (`"blob": [offset, length]`) instead of giving `data`.
//!
//! The format has one version, [`VERSION`] (1): it is the only one written and the only one read.
//! A file with any other `version` is refused ([`FormatError::TooNew`] for a newer one,
//! [`FormatError::Unsupported`] otherwise). Saving writes indented JSON with a stable member order
//! (readable, diffable); compressed files are compact.
//!
//! Native files must carry Kition Vector's own format tag. Other product tags are rejected.
#![forbid(unsafe_code)]

#[cfg(not(target_arch = "wasm32"))]
mod atomic;

use std::borrow::Cow;
use std::collections::{BTreeMap, HashSet};
use std::io::{Read as _, Write as _};

use kition_vector_doc::{Document, ImageBlob};
use serde::{Deserialize, Serialize};

#[cfg(not(target_arch = "wasm32"))]
pub use atomic::{write_atomic, write_atomic_with, write_new_with};

/// The format version: the only one written and read.
pub const VERSION: u32 = 1;
/// Data this large or larger goes after the JSON, not into it as base64.
pub const INLINE_MAX: usize = 8 << 10;
/// What separates the JSON from the data after it.
pub const BLOB_MAGIC: &[u8] = b"\n\0VCBLOBS\0";
/// Most pixels on the longer side of an embedded preview.
pub const PREVIEW_MAX: u32 = 256;
pub const EXTENSION: &str = "kvector";
/// Most bytes a compressed file may unpack to (a guard against decompression bombs).
const MAX_UNPACKED: u64 = 512 << 20;

/// Is `ext` (without the dot, any case) a native document extension?
pub fn is_native_ext(ext: &str) -> bool {
    ext.eq_ignore_ascii_case(EXTENSION)
}

/// Does the file name or path end in a native document extension?
pub fn is_native_name(name: &str) -> bool {
    std::path::Path::new(name).extension().and_then(|e| e.to_str()).is_some_and(is_native_ext)
}

#[derive(Debug, thiserror::Error)]
pub enum FormatError {
    #[error("not a Kition Vector file: {0}")]
    NotKitionVector(String),
    #[error("file version {0} is newer than this Kition Vector supports ({VERSION})")]
    TooNew(u32),
    #[error("file version {0} isn't a Kition Vector format version (only {VERSION} is)")]
    Unsupported(u32),
    #[error("invalid image data for `{0}`")]
    BadImage(String),
    #[error("the compressed file unpacks to more than {} MB", MAX_UNPACKED >> 20)]
    TooBig,
    #[error("can't encode the document: {0}")]
    Encode(String),
}

#[derive(Serialize, Deserialize)]
struct Image {
    mime: String,
    /// The bytes as base64, unless they come after the JSON (`blob`).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    data: String,
    /// Where the bytes are in the data after the JSON: `[offset, length]`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    blob: Option<[u64; 2]>,
    /// `data` is a linked image's preview ([`ImageBlob::proxy`]), not the file's bytes.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    proxy: bool,
}

impl Image {
    /// Its bytes: the base64 `data`, or its part of `tail` (the data after the JSON).
    fn bytes(&self, tail: &[u8]) -> Option<Vec<u8>> {
        match self.blob {
            Some([offset, len]) => {
                let start = usize::try_from(offset).ok()?;
                let end = start.checked_add(usize::try_from(len).ok()?)?;
                tail.get(start..end).map(<[u8]>::to_vec)
            }
            None => base64_decode(&self.data),
        }
    }
}

/// Entries written for one file: large bytes go to the data after the JSON.
struct Blobs {
    tail: Vec<u8>,
}

impl Blobs {
    fn image(&mut self, mime: &str, bytes: &[u8], proxy: bool) -> Image {
        if bytes.len() >= INLINE_MAX {
            let at = self.tail.len() as u64;
            self.tail.extend_from_slice(bytes);
            return Image { mime: mime.into(), data: String::new(), blob: Some([at, bytes.len() as u64]), proxy };
        }
        Image { mime: mime.into(), data: base64_encode(bytes), blob: None, proxy }
    }
}

/// The MIME type of a preview's bytes (JPEG or PNG).
fn preview_mime(bytes: &[u8]) -> &'static str {
    if bytes.starts_with(&[0xff, 0xd8]) { "image/jpeg" } else { kition_vector_doc::links::PROXY_MIME }
}

/// The JSON of a file and the data after it, split at [`BLOB_MAGIC`] after the first JSON
/// value.
fn split_tail(text: &[u8]) -> (&[u8], &[u8]) {
    let mut values = serde_json::Deserializer::from_slice(text).into_iter::<serde::de::IgnoredAny>();
    if values.next().is_some_and(|v| v.is_ok()) {
        let end = values.byte_offset();
        if let Some(rest) = text.get(end..)
            && rest.starts_with(BLOB_MAGIC)
        {
            return (text.get(..end).unwrap_or(text), rest.get(BLOB_MAGIC.len()..).unwrap_or_default());
        }
    }
    (text, &[])
}

#[derive(Deserialize)]
struct File {
    format: String,
    version: u32,
    document: Document,
    #[serde(default)]
    images: BTreeMap<String, Image>,
    #[serde(default)]
    profiles: BTreeMap<String, Image>,
}

/// How [`save_with`] writes a document.
#[derive(Clone, Debug)]
pub struct SaveOptions {
    /// Indented JSON (readable, diffable; the default); compact otherwise, for a native document
    /// carried inside another file. Compressed files are always compact.
    pub pretty: bool,
    /// gzip the file (Use Compression).
    pub compress: bool,
    /// A PNG preview to embed (at most [`PREVIEW_MAX`] pixels on its longer side).
    pub preview: Option<Vec<u8>>,
    /// Include Linked Files: linked images keep the file's own bytes, not just their preview.
    pub include_linked: bool,
    /// Embed ICC Profiles: ICC files by profile name.
    pub profiles: BTreeMap<String, Vec<u8>>,
    /// Create PDF-Compatible File: a PDF of the document to carry.
    pub pdf: Option<Vec<u8>>,
}

impl Default for SaveOptions {
    fn default() -> Self {
        Self { pretty: true, compress: false, preview: None, include_linked: false, profiles: BTreeMap::new(), pdf: None }
    }
}

/// The MIME type of embedded ICC profiles.
pub const ICC_MIME: &str = "application/vnd.iccprofile";

/// Serialize a document (pretty = human-diffable). Editing-mode working copies are left out
/// ([`Document::without_edit_modes`]).
pub fn save(doc: &Document, pretty: bool) -> Vec<u8> {
    save_with(doc, &SaveOptions { pretty, ..SaveOptions::default() }).unwrap_or_default()
}

/// Serialize for saving to disk: pretty-printed, whatever the document's size.
pub fn save_file(doc: &Document) -> Vec<u8> {
    save_with(doc, &SaveOptions::default()).unwrap_or_default()
}

/// Serialize a document as `o` says: compression, layout and preview. Editing-mode working copies,
/// resources only exporting adds and images nothing uses are left out.
pub fn save_with(doc: &Document, o: &SaveOptions) -> Result<Vec<u8>, FormatError> {
    let pretty = o.pretty && !o.compress;
    let mut d = doc.without_edit_modes().into_owned();
    d.drop_placed_resources();
    let linked = d.linked_only_images();
    // The blobs go in the file's `images` (only those the document uses).
    let blobs = std::mem::take(&mut d.images);
    let body = to_json(&d, pretty)?;
    // Images are only referenced by image objects' `key` (wherever the objects are: layers,
    // symbols, patterns, masks, or foreign data).
    let used = if blobs.is_empty() { HashSet::new() } else { string_members(&body, "key") };
    let mut out_blobs = Blobs { tail: vec![] };
    let images: BTreeMap<&str, Image> = blobs
        .iter()
        .filter(|(k, _)| used.contains(k.as_str()))
        .map(|(k, b)| {
            // Include Linked Files keeps the file's bytes whenever they are loaded.
            let preview_only = linked.contains(k) && (!o.include_linked || b.is_proxy());
            let image = match b.proxy.as_ref().filter(|_| preview_only) {
                Some(p) => out_blobs.image(preview_mime(p), p, true),
                None => out_blobs.image(&b.mime, &b.bytes, false),
            };
            (k.as_str(), image)
        })
        .collect();
    let profiles: BTreeMap<&str, Image> = o.profiles.iter().map(|(name, icc)| (name.as_str(), out_blobs.image(ICC_MIME, icc, false))).collect();
    let pdf = o.pdf.as_deref().map(|p| out_blobs.image("application/pdf", p, false));
    let blob = |mime: &str, bytes: &[u8]| Image { mime: mime.into(), data: base64_encode(bytes), blob: None, proxy: false };
    let carried = profiles.values().chain(&pdf).map(|i| i.data.len()).sum::<usize>();
    let size = body.len() + images.values().map(|i| i.data.len()).sum::<usize>() + carried + 256;
    let mut w = Envelope { out: Vec::with_capacity(size), pretty };
    w.field("format", &EXTENSION)?;
    w.field("version", &VERSION)?;
    w.field("generator", &format!("Kition Vector {}", env!("CARGO_PKG_VERSION")))?;
    if let Some(png) = &o.preview {
        w.field("preview", &blob("image/png", png))?;
    }
    if !profiles.is_empty() {
        w.field("profiles", &profiles)?;
    }
    if let Some(pdf) = &pdf {
        w.field("pdf", pdf)?;
    }
    w.raw("document", &body);
    w.field("images", &images)?;
    let mut out = w.finish();
    if !out_blobs.tail.is_empty() {
        out.extend_from_slice(BLOB_MAGIC);
        out.extend_from_slice(&out_blobs.tail);
    }
    if !o.compress {
        return Ok(out);
    }
    let mut gz = flate2::write::GzEncoder::new(Vec::with_capacity(out.len() / 4), flate2::Compression::default());
    gz.write_all(&out).map_err(encode_err)?;
    gz.finish().map_err(encode_err)
}

fn encode_err(e: impl std::fmt::Display) -> FormatError {
    FormatError::Encode(e.to_string())
}

fn to_json(v: &impl Serialize, pretty: bool) -> Result<Vec<u8>, FormatError> {
    if pretty { serde_json::to_vec_pretty(v) } else { serde_json::to_vec(v) }.map_err(encode_err)
}

/// The file's top-level object, written member by member (the document goes in pre-serialized).
struct Envelope {
    out: Vec<u8>,
    pretty: bool,
}

impl Envelope {
    fn field(&mut self, key: &str, value: &impl Serialize) -> Result<(), FormatError> {
        let json = to_json(value, self.pretty)?;
        self.raw(key, &json);
        Ok(())
    }

    /// Member `key` (a plain name) with the JSON text `json`, indented one level when pretty (as
    /// serde_json would).
    fn raw(&mut self, key: &str, json: &[u8]) {
        let out = &mut self.out;
        out.push(if out.is_empty() { b'{' } else { b',' });
        if self.pretty {
            out.extend_from_slice(b"\n  ");
        }
        out.push(b'"');
        out.extend_from_slice(key.as_bytes());
        out.extend_from_slice(if self.pretty { b"\": " } else { b"\":" });
        // Newlines only occur between tokens (inside strings they are escaped).
        let mut lines = json.split(|&b| b == b'\n');
        out.extend_from_slice(lines.next().unwrap_or_default());
        for line in lines {
            out.extend_from_slice(b"\n  ");
            out.extend_from_slice(line);
        }
    }

    fn finish(mut self) -> Vec<u8> {
        self.out.extend_from_slice(if self.pretty { b"\n}" } else { b"}" });
        self.out
    }
}

/// The string values of every `"<name>": "…"` member at any depth of JSON text. Quotes inside JSON
/// strings are escaped, so every match is a real member.
fn string_members(json: &[u8], name: &str) -> HashSet<String> {
    let tag = format!("\"{name}\":");
    let tag = tag.as_bytes();
    let mut out = HashSet::new();
    let mut rest = json;
    while let Some(at) = rest.windows(tag.len()).position(|w| w == tag) {
        rest = rest.get(at + tag.len()..).unwrap_or_default();
        let value = rest.trim_ascii_start();
        if value.first() == Some(&b'"')
            && let Some(Ok(s)) = serde_json::Deserializer::from_slice(value).into_iter::<String>().next()
        {
            out.insert(s);
        }
    }
    out
}

/// Is this gzip data (a compressed file)?
pub fn is_compressed(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0x1f, 0x8b])
}

/// The JSON text of a file, unpacked when compressed.
fn unpack(bytes: &[u8]) -> Result<Cow<'_, [u8]>, FormatError> {
    if !is_compressed(bytes) {
        return Ok(Cow::Borrowed(bytes));
    }
    let mut raw = Vec::new();
    flate2::read::GzDecoder::new(bytes)
        .take(MAX_UNPACKED + 1)
        .read_to_end(&mut raw)
        .map_err(|e| FormatError::NotKitionVector(format!("damaged compressed data: {e}")))?;
    if raw.len() as u64 > MAX_UNPACKED {
        return Err(FormatError::TooBig);
    }
    Ok(Cow::Owned(raw))
}

pub fn load(bytes: &[u8]) -> Result<Document, FormatError> {
    load_file(bytes).map(|f| f.doc)
}

/// A native file read: its document and the colour profiles it carries.
#[derive(Debug)]
pub struct NativeFile {
    pub doc: Document,
    /// ICC files by profile name (saved with Embed ICC Profiles; undecodable ones are left out).
    pub profiles: BTreeMap<String, Vec<u8>>,
}

/// [`load`], with everything else the file carries.
pub fn load_file(bytes: &[u8]) -> Result<NativeFile, FormatError> {
    let text = unpack(bytes)?;
    let (json, tail) = split_tail(&text);
    let f: File = serde_json::from_slice(json).map_err(|e| FormatError::NotKitionVector(e.to_string()))?;
    if f.format != EXTENSION {
        return Err(FormatError::NotKitionVector(format!("format is `{}`", f.format)));
    }
    if f.version > VERSION {
        return Err(FormatError::TooNew(f.version));
    }
    if f.version != VERSION {
        return Err(FormatError::Unsupported(f.version));
    }
    let mut doc = f.document;
    for (k, img) in f.images {
        let bytes = img.bytes(tail).ok_or_else(|| FormatError::BadImage(k.clone()))?;
        let mut blob = ImageBlob::new(img.mime, bytes);
        // Until the linked file is read, the preview stands in for it.
        if img.proxy {
            blob.proxy = Some(blob.bytes.clone());
        }
        doc.images.insert(k, blob);
    }
    // Saved mid-edit by an older version: drop the opacity-mask editing layer.
    doc.drop_edit_modes();
    // Saved before per-fill/stroke overprint: the Overprint Black list becomes overprint flags.
    doc.migrate_overprint_black();
    // Saved before v3: the assigned profiles move out of `unknown`.
    doc.migrate_color_profiles();
    doc.fix_next_id();
    // Untrusted: a name twice in a table would make a write by that name pick one (C8-vector-4).
    doc.unique_names();
    // Untrusted: capped, named and pointing at objects the file has.
    doc.tidy_saved_selections();
    let profiles = f.profiles.into_iter().filter_map(|(name, icc)| Some((name, icc.bytes(tail)?))).collect();
    Ok(NativeFile { doc, profiles })
}

/// The members of a native file besides its document (which is skipped, not decoded).
#[derive(Deserialize)]
struct Head {
    preview: Option<Image>,
    pdf: Option<Image>,
}

/// The file's head and the data after its JSON.
fn head(bytes: &[u8]) -> Option<(Head, Vec<u8>)> {
    let text = unpack(bytes).ok()?;
    let (json, tail) = split_tail(&text);
    Some((serde_json::from_slice(json).ok()?, tail.to_vec()))
}

/// The preview PNG embedded in a native file (`None`: it has none, or isn't a native file).
pub fn preview(bytes: &[u8]) -> Option<Vec<u8>> {
    let (h, tail) = head(bytes)?;
    h.preview?.bytes(&tail)
}

/// The PDF a native file saved with Create PDF-Compatible File carries (`None`: it has none, or
/// isn't a native file).
pub fn pdf_content(bytes: &[u8]) -> Option<Vec<u8>> {
    let (h, tail) = head(bytes)?;
    h.pdf?.bytes(&tail)
}

/// Does this look like a `.kvector` file (compressed or not)?
pub fn sniff(bytes: &[u8]) -> bool {
    const HEAD: usize = 256;
    let head: Cow<'_, [u8]> = if is_compressed(bytes) {
        let mut head = Vec::with_capacity(HEAD);
        // A damaged stream may still unpack its start: judge what came out.
        let _ = flate2::read::GzDecoder::new(bytes).take(HEAD as u64).read_to_end(&mut head);
        Cow::Owned(head)
    } else {
        Cow::Borrowed(bytes.get(..HEAD).unwrap_or(bytes))
    };
    // The cut may split a character.
    let s = String::from_utf8_lossy(&head);
    s.trim_start().starts_with('{') && s.contains(&format!("\"{EXTENSION}\""))
}

const B64: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

pub fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for c in data.chunks(3) {
        let n = (c[0] as u32) << 16 | (*c.get(1).unwrap_or(&0) as u32) << 8 | *c.get(2).unwrap_or(&0) as u32;
        out.push(B64[(n >> 18) as usize & 63] as char);
        out.push(B64[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { B64[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { B64[n as usize & 63] as char } else { '=' });
    }
    out
}

pub fn base64_decode(s: &str) -> Option<Vec<u8>> {
    let val = |c: u8| B64.iter().position(|&b| b == c).map(|p| p as u32);
    let clean: Vec<u8> = s.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if !clean.len().is_multiple_of(4) {
        return None;
    }
    let mut out = Vec::with_capacity(clean.len() / 4 * 3);
    for c in clean.chunks(4) {
        let mut n = 0u32;
        let mut pad = 0;
        for &b in c {
            n <<= 6;
            if b == b'=' {
                pad += 1;
            } else {
                n |= val(b)?;
            }
        }
        out.push((n >> 16) as u8);
        if pad < 2 {
            out.push((n >> 8) as u8);
        }
        if pad < 1 {
            out.push(n as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_native_extension_and_format_are_recognised() {
        assert!(is_native_name("old/Poster.kvector") && is_native_name("new.kvector") && !is_native_name("x.svg"));
        assert!(load(br#"{"format":"other","version":1,"document":{}}"#).is_err());
        assert!(load(br#"{"format":"otherformat","version":1,"document":{}}"#).is_err());
    }

    #[test]
    fn only_version_1_is_written_and_read() {
        let d = Document::new(10.0, 10.0);
        let current = save(&d, false);
        let v: serde_json::Value = serde_json::from_slice(&current).unwrap();
        assert_eq!((VERSION, v["version"].as_u64()), (1, Some(1)));
        assert!(load(&current).is_ok());
        let text = String::from_utf8(current).unwrap();
        for other in [0, 2, 3, 4] {
            let changed = text.replacen("\"version\":1", &format!("\"version\":{other}"), 1);
            let r = load(changed.as_bytes());
            assert!(matches!(r, Err(FormatError::Unsupported(n) | FormatError::TooNew(n)) if n == other), "version {other}: {r:?}");
        }
        assert!(matches!(load(text.replacen("\"version\":1", "\"version\":0", 1).as_bytes()), Err(FormatError::Unsupported(0))));
    }

    #[test]
    fn saving_to_disk_is_pretty_printed() {
        let mut d = Document::new(10.0, 10.0);
        let l = d.layers[0].id;
        for _ in 0..3000 {
            let id = d.alloc_id();
            d.insert(Some(l), 0, Node::path(id, shapes::ellipse(Rect::new(0.0, 0.0, 1.0, 1.0)), Appearance::default_art())).unwrap();
        }
        let text = String::from_utf8(save_file(&d)).unwrap();
        assert!(text.starts_with("{\n  \"format\": \"kvector\",\n  \"version\": 1,"), "{}", &text[..80]);
        assert_eq!(load(text.as_bytes()).unwrap().node_count(), d.node_count());
        // Compressed files stay compact inside.
        let packed = save_with(&d, &SaveOptions { compress: true, ..SaveOptions::default() }).unwrap();
        let mut inner = Vec::new();
        std::io::Read::read_to_end(&mut flate2::read::GzDecoder::new(&packed[..]), &mut inner).unwrap();
        assert!(inner.starts_with(b"{\"format\":\"kvector\",\"version\":1,"));
    }
    use kition_vector_doc::{Appearance, ImageObject, Node, NodeKind};
    use kition_vector_geom::{Affine, Rect, shapes};

    #[test]
    fn roundtrip() {
        let mut d = Document::new(100.0, 100.0);
        let l = d.layers[0].id;
        let id = d.alloc_id();
        d.insert(Some(l), 0, Node::path(id, shapes::ellipse(Rect::new(0.0, 0.0, 10.0, 10.0)), Appearance::default_art())).unwrap();
        d.images.insert("k".into(), ImageBlob::new("image/png", vec![1, 2, 3, 250]));
        // Only images in use are saved.
        let img = d.alloc_id();
        let im = ImageObject { key: "k".into(), width: 1, height: 1, xf: Affine::IDENTITY, link: None, placement: Default::default() };
        d.insert(Some(l), 1, Node::new(img, NodeKind::Image(im))).unwrap();
        let bytes = save(&d, true);
        assert!(sniff(&bytes));
        let back = load(&bytes).unwrap();
        assert_eq!(back.node_count(), d.node_count());
        assert_eq!(back.images["k"].bytes.as_slice(), &[1, 2, 3, 250]);
        assert_eq!(back.node(id).unwrap().path_data(), d.node(id).unwrap().path_data());
    }

    #[test]
    fn rejects_foreign_and_future() {
        assert!(load(b"{}").is_err());
        let d = Document::new(1.0, 1.0);
        let mut v: serde_json::Value = serde_json::from_slice(&save(&d, false)).unwrap();
        v["version"] = 99.into();
        assert!(matches!(load(&serde_json::to_vec(&v).unwrap()), Err(FormatError::TooNew(99))));
    }

    #[test]
    fn base64() {
        for s in [&b""[..], b"f", b"fo", b"foo", b"foob", b"fooba", b"foobar"] {
            assert_eq!(base64_decode(&base64_encode(s)).unwrap(), s);
        }
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }
}
